use std::{collections::BTreeMap, ops::Range};

use crate::{
    Bindings, Budget, EvaluationError, Located, Location, NativeArguments, NativeExpression,
    Predicate, PredicateKind, ProjectionHost, QueryError, QueryErrorCategory, Truth,
    evaluate_expression,
};

/// Ordered native fields. Values retain their host handles and source coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeCapture<V> {
    pub fields: Vec<(String, V)>,
}

impl<V> NativeCapture<V> {
    /// Read a written field, charging comparisons before borrowing its value.
    ///
    /// # Errors
    /// A missing field or exhausted allowance fails at the field's query range.
    pub fn get(
        &self,
        field: &Located<impl AsRef<str>>,
        budget: &mut Budget,
    ) -> Result<&V, QueryError> {
        for (name, value) in &self.fields {
            budget.charge(
                1 + name.len().min(field.value.as_ref().len()),
                Location::Query(field.range.clone()),
            )?;
            if name == field.value.as_ref() {
                return Ok(value);
            }
        }
        Err(QueryError::query(
            field.range.clone(),
            "unknown native capture field",
        ))
    }
}

/// Finite native clauses execute in written order; later clauses see earlier names.
#[derive(Debug, PartialEq, Eq)]
pub struct NativeMatch<Q, P> {
    pub clauses: Vec<Located<MatchClause<Q, P>>>,
    pub range: Range<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MatchClause<Q, P> {
    From { name: Located<String>, source: Q },
    Bind { name: Located<String>, value: Q },
    Where { predicate: Predicate<P> },
}

/// Preflight every clause without obtaining subjects, including unreachable ones.
/// Returned native field kinds preserve declaration order and empty-input facts.
///
/// # Errors
/// Own-source/forward references, ambient or duplicate names, invalid native
/// types/effects, malformed guards and resource exhaustion retain query ranges.
pub fn match_types<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<NativeCapture<H::Kind>, QueryError> {
    budget.within_evaluation(1, Location::Query(matching.range.clone()), |budget| {
        budget.check_evaluation_depth(
            matching.clauses.len(),
            matching.clauses.len(),
            Location::Query(matching.range.clone()),
        )?;
        let mut scope = crate::projection::copy_scope::<C, H>(scope, budget, &matching.range)?;
        slots::<(String, H::Kind)>(matching.clauses.len(), budget, &matching.range, false)?;
        let mut fields = Vec::with_capacity(matching.clauses.len());
        preflight_clauses(matching, 0, host, input, &mut scope, &mut fields, budget)?;
        Ok(NativeCapture { fields })
    })
}

fn preflight_clauses<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    index: usize,
    host: &H,
    input: &H::Kind,
    scope: &mut BTreeMap<String, H::Kind>,
    fields: &mut Vec<(String, H::Kind)>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let range = matching
        .clauses
        .get(index)
        .map_or(&matching.range, |clause| &clause.range);
    budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
        let Some(clause) = matching.clauses.get(index) else {
            return Ok(());
        };
        budget.charge(1, Location::Query(clause.range.clone()))?;
        match &clause.value {
            MatchClause::From { name, source }
            | MatchClause::Bind {
                name,
                value: source,
            } => {
                if name.value.is_empty() || scope.contains_key(&name.value) || name.value == "input"
                {
                    return Err(QueryError::query(
                        name.range.clone(),
                        "capture name is empty, reserved or already defined",
                    ));
                }
                // Insert after preflight: a capture is absent in its own operand.
                let kind = host.expression_kind(source.query(), input, scope, budget)?;
                let kind = host.subject_kind(&kind, budget, &source.value().range)?;
                budget.charge(
                    name.value
                        .len()
                        .checked_mul(2)
                        .and_then(|n| n.checked_add(1))
                        .ok_or_else(|| {
                            QueryError::query(name.range.clone(), "capture metadata size overflow")
                        })?,
                    Location::Query(name.range.clone()),
                )?;
                fields.push((name.value.clone(), kind.clone()));
                scope.insert(name.value.clone(), kind);
            }
            MatchClause::Where { predicate } => {
                validate_guard(predicate, host, input, scope, budget)?
            }
        }

        preflight_clauses(matching, index + 1, host, input, scope, fields, budget)
    })
}

fn validate_guard<C, H: ProjectionHost<C>>(
    predicate: &Predicate<H::Stage>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    crate::eval::validate_predicate_leaves(predicate, budget, |guard, budget, range| {
        host.validate_guard_leaf(guard, input, scope, budget, range)
    })
}

pub(crate) fn dispose_predicate<P>(predicate: Predicate<P>) {
    let mut pending = vec![predicate];
    while let Some(predicate) = pending.pop() {
        match predicate.value {
            PredicateKind::Leaf(_) => {}
            PredicateKind::Not(child) | PredicateKind::Group(child) => pending.push(*child),
            PredicateKind::And(children) | PredicateKind::Or(children) => pending.extend(children),
        }
    }
}

/// Execute finite captures with one allowance, in depth-first declaration order.
/// The rightmost `from` varies fastest. `bind` requires exactly one subject;
/// false skips a candidate, while Unknown refuses the required guard.
///
/// # Errors
/// All clauses preflight before enumeration, even after empty sources/false guards.
/// Native failures retain their type; results, field copies and nesting are bounded.
pub fn evaluate_match<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
    evaluate_match_context(
        matching,
        host,
        input,
        &mut MatchContext::Direct { context, bindings },
        None,
        budget,
    )
}

/// Execute the same dependent clause chain with an active function argument
/// runner. Scoped argument metadata declares earlier capture names before any
/// source executes; false/Unknown guards retain ordinary matching behavior.
///
/// # Errors
/// Source, guard, function and resource failures retain their original locations.
pub fn evaluate_match_with_arguments<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    host: &H,
    input: H::Value,
    arguments: &mut dyn NativeArguments<C, H>,
    budget: &mut Budget,
) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
    let schema = prepared_schema(matching, host, &input, arguments, budget)?;
    evaluate_prepared_match(matching, &schema, host, input, arguments, budget)
}
pub(crate) fn prepared_schema<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    host: &H,
    input: &H::Value,
    arguments: &mut dyn NativeArguments<C, H>,
    budget: &mut Budget,
) -> Result<NativeCapture<H::Kind>, QueryError> {
    let mut fields = Vec::new();
    for clause in &matching.clauses {
        if let MatchClause::From { name, source }
        | MatchClause::Bind {
            name,
            value: source,
        } = &clause.value
        {
            let kind = if let Some(kind) = arguments.argument_kind(source.value()) {
                kind
            } else {
                let input_kind = host.kind(input, budget, &matching.range)?;
                let mut scope = crate::projection::scope_kinds(
                    host,
                    arguments.parts().1,
                    budget,
                    &matching.range,
                )?;
                scope.extend(fields.iter().cloned());
                host.expression_kind(source.query(), &input_kind, &scope, budget)?
            };
            let kind = host.subject_kind(&kind, budget, &source.value().range)?;
            budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
            slots::<(String, H::Kind)>(1, budget, &matching.range, false)?;
            fields.push((name.value.clone(), kind));
        }
    }
    let _ = input;
    Ok(NativeCapture { fields })
}
pub(crate) fn evaluate_prepared_match<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    schema: &NativeCapture<H::Kind>,
    host: &H,
    input: H::Value,
    arguments: &mut dyn NativeArguments<C, H>,
    budget: &mut Budget,
) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
    evaluate_match_context(
        matching,
        host,
        input,
        &mut MatchContext::Arguments(arguments),
        Some(schema),
        budget,
    )
}

enum MatchContext<'a, C, H: ProjectionHost<C>> {
    Direct {
        context: &'a mut C,
        bindings: &'a Bindings<H::Value>,
    },
    Arguments(&'a mut dyn NativeArguments<C, H>),
}
impl<C, H: ProjectionHost<C>> MatchContext<'_, C, H> {
    fn parts(&mut self) -> (&mut C, &Bindings<H::Value>) {
        match self {
            Self::Direct { context, bindings } => (context, bindings),
            Self::Arguments(arguments) => arguments.parts(),
        }
    }
    fn source(
        &mut self,
        host: &H,
        source: &crate::NativeBody<NativeExpression<C, H>>,
        input: H::Value,
        bindings: &Bindings<H::Value>,
        budget: &mut Budget,
    ) -> Result<H::Value, EvaluationError<H::Error>> {
        match self {
            Self::Direct { context, .. } => {
                evaluate_expression(source.query(), host, input, context, bindings, budget)
            }
            Self::Arguments(arguments) => {
                arguments.evaluate_body(source, Some(input), Some(bindings), budget)
            }
        }
    }
    fn guard(
        &mut self,
        host: &H,
        guard: &H::Stage,
        input: &H::Value,
        bindings: &Bindings<H::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<H::Error>> {
        match self {
            Self::Direct { context, .. } => {
                host.evaluate_guard_leaf(guard, input, context, bindings, budget, range)
            }
            Self::Arguments(arguments) => host.evaluate_guard_leaf_with_arguments(
                guard, input, bindings, *arguments, budget, range,
            ),
        }
    }
}

fn evaluate_match_context<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    host: &H,
    input: H::Value,
    context: &mut MatchContext<'_, C, H>,
    prepared_schema: Option<&NativeCapture<H::Kind>>,
    budget: &mut Budget,
) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
    if budget.remaining_results().is_none() {
        return Err(QueryError::query(
            matching.range.clone(),
            "native matching requires aggregate result limits",
        )
        .into());
    }
    budget.within_evaluation(1, Location::Query(matching.range.clone()), |budget| {
        let input_kind = host.kind(&input, budget, &matching.range)?;
        let bindings =
            context
                .parts()
                .1
                .with_shared_values(BTreeMap::new(), budget, &matching.range)?;
        let scope = crate::projection::scope_kinds(host, &bindings, budget, &matching.range)?;
        let discovered;
        let schema = if let Some(schema) = prepared_schema {
            schema
        } else {
            discovered = match_types(matching, host, &input_kind, &scope, budget)?;
            &discovered
        };
        slots::<H::Value>(1, budget, &matching.range, true)?;
        let mut rows = Vec::new();
        run_clauses(
            matching, 0, schema, host, &input, context, &bindings, budget, &mut rows,
        )?;
        Ok(rows)
    })
}

// Recursion represents dependent environments and is capped before execution
// and at every active call. No Cartesian candidate collection is materialized.
#[allow(
    clippy::too_many_arguments,
    reason = "one bounded clause chain shares the request state"
)]
fn run_clauses<C, H: ProjectionHost<C>>(
    matching: &NativeMatch<crate::NativeBody<NativeExpression<C, H>>, H::Stage>,
    index: usize,
    schema: &NativeCapture<H::Kind>,
    host: &H,
    input: &H::Value,
    context: &mut MatchContext<'_, C, H>,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
    rows: &mut Vec<H::Value>,
) -> Result<(), EvaluationError<H::Error>> {
    let range = matching
        .clauses
        .get(index)
        .map_or(&matching.range, |clause| &clause.range);
    budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
        budget.charge(1, Location::Query(range.clone()))?;
        let Some(clause) = matching.clauses.get(index) else {
            check_slots::<H::Value>(
                rows.len().checked_add(1).ok_or_else(|| {
                    QueryError::query(range.clone(), "capture allocation size overflow")
                })?,
                range,
            )?;
            budget.check_items(
                rows.len().checked_add(1).ok_or_else(|| {
                    QueryError::query(range.clone(), "item limit exceeded")
                        .with_category(QueryErrorCategory::ItemLimit)
                })?,
                Location::Query(range.clone()),
            )?;
            slots::<(String, H::Value)>(schema.fields.len(), budget, range, true)?;
            slots::<H::Value>(1, budget, range, true)?;
            let mut fields = Vec::with_capacity(schema.fields.len());
            for (name, _) in &schema.fields {
                budget.charge(name.len(), Location::Query(range.clone()))?;
                let value = host.copy_value(
                    bindings.get(name, range.clone())?,
                    context.parts().0,
                    budget,
                    range,
                )?;
                fields.push((name.clone(), value));
            }
            rows.push(host.construct_record(fields, context.parts().0, budget, range)?);
            return Ok(());
        };
        match &clause.value {
            MatchClause::From { name, source }
            | MatchClause::Bind {
                name,
                value: source,
            } => {
                let relative =
                    host.copy_value(input, context.parts().0, budget, &source.value().range)?;
                let value = context.source(host, source, relative, bindings, budget)?;
                let subjects =
                    host.subjects(value, context.parts().0, budget, &source.value().range)?;
                budget.check_items(
                    subjects.len(),
                    Location::Query(source.value().range.clone()),
                )?;
                if matches!(clause.value, MatchClause::Bind { .. }) && subjects.len() != 1 {
                    return Err(QueryError::query(
                        source.value().range.clone(),
                        "required capture needs exactly one value",
                    )
                    .into());
                }
                for subject in subjects {
                    // Candidate admission precedes every later guard, including false.
                    budget.charge(1, Location::Query(clause.range.clone()))?;
                    budget.charge(name.value.len(), Location::Query(name.range.clone()))?;
                    let local = bindings.with_local(name.clone(), subject, budget)?;
                    run_clauses(
                        matching,
                        index + 1,
                        schema,
                        host,
                        input,
                        context,
                        &local,
                        budget,
                        rows,
                    )?;
                }
            }
            MatchClause::Where { predicate } => {
                let truth = crate::eval::evaluate_predicate(
                    predicate,
                    crate::Nesting::Nodes,
                    budget,
                    |guard, budget, range| {
                        context.guard(host, guard, input, bindings, budget, range)
                    },
                )?;
                if host.require_guard(
                    truth,
                    context.parts().0,
                    budget,
                    &predicate.range,
                    "required match guard is Unknown",
                )? {
                    run_clauses(
                        matching,
                        index + 1,
                        schema,
                        host,
                        input,
                        context,
                        bindings,
                        budget,
                        rows,
                    )?
                }
            }
        }
        Ok(())
    })
}

fn slots<T>(
    count: usize,
    budget: &mut Budget,
    range: &Range<usize>,
    results: bool,
) -> Result<(), QueryError> {
    check_slots::<T>(count, range)?;
    budget.charge(count, Location::Query(range.clone()))?;
    if results {
        budget.charge_results(count, Location::Query(range.clone()))?;
    }
    Ok(())
}

fn check_slots<T>(count: usize, range: &Range<usize>) -> Result<(), QueryError> {
    if count
        .checked_mul(std::mem::size_of::<T>())
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "capture allocation size overflow",
        ));
    }
    Ok(())
}

/// Release native operands and Boolean wrappers iteratively. Hosts remain
/// responsible for releasing their own opaque stage and guard payloads.
pub fn dispose_match<S, O, E, P>(
    matching: NativeMatch<crate::NativeBody<crate::Expression<S, O, E>>, P>,
) {
    for clause in matching.clauses {
        match clause.value {
            MatchClause::From { source, .. } | MatchClause::Bind { value: source, .. } => {
                crate::dispose_native_value(source.into_value())
            }
            MatchClause::Where { predicate } => {
                dispose_predicate(predicate);
            }
        }
    }
}
