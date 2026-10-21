//! Preparation and execution of common native composition.
use crate::*;
use std::{collections::BTreeMap, ops::Range};

/// One native body retained as a declared argument of the existing prepared runner.
/// Ordinary value expressions keep their scalar projection behavior; composition
/// executes `query()` directly and retains its native result.
#[derive(Debug, PartialEq)]
pub struct NativeBody<Q> {
    value: ValueExpression<Q>,
}
impl<Q> NativeBody<Q> {
    pub fn new(query: Q, range: Range<usize>) -> Self {
        Self {
            value: Located::new(ValueExpressionKind::NativeValue(Box::new(query)), range),
        }
    }
    pub fn query(&self) -> &Q {
        let ValueExpressionKind::NativeValue(query) = &self.value.value else {
            unreachable!("native body representation")
        };
        query
    }
    pub fn value(&self) -> &ValueExpression<Q> {
        &self.value
    }
    pub fn into_value(self) -> ValueExpression<Q> {
        self.value
    }
}

/// Filtering yields a selection even when its input was one singleton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionShape {
    Preserve,
    Filter,
}

pub(crate) fn is_common<S, E, O>(step: &ExpressionStep<S, E, O>) -> bool {
    matches!(
        step,
        ExpressionStep::Group(_)
            | ExpressionStep::Filter(_)
            | ExpressionStep::Match(_)
            | ExpressionStep::Ordering(_)
            | ExpressionStep::Mapping(_)
            | ExpressionStep::MapSorting
            | ExpressionStep::Collection(_)
            | ExpressionStep::Head(_)
            | ExpressionStep::Tail(_)
            | ExpressionStep::Check { .. }
            | ExpressionStep::WhereExists(_)
            | ExpressionStep::FixedPoint(_)
            | ExpressionStep::Paths(_)
    )
}

pub(crate) fn explicit_source<S, E, O>(step: &ExpressionStep<S, E, O>) -> bool {
    match step {
        ExpressionStep::Value(_) => true,
        ExpressionStep::Group(body) => explicit_expression(body.query()),
        ExpressionStep::Match(matching) => {
            matching.clauses.iter().all(|clause| match &clause.value {
                MatchClause::From { source, .. } | MatchClause::Bind { value: source, .. } => {
                    explicit_expression(source.query())
                }
                MatchClause::Where { .. } => true,
            })
        }
        _ => false,
    }
}
fn explicit_expression<S, O, E>(expression: &Expression<S, O, E>) -> bool {
    match &expression.value {
        ExpressionKind::Binding(_) => true,
        ExpressionKind::Group(value) | ExpressionKind::Field { value, .. } => {
            explicit_expression(value)
        }
        ExpressionKind::Sequence { input, steps } => {
            steps
                .first()
                .is_some_and(|step| explicit_source(&step.value))
                || explicit_expression(input)
        }
        _ => false,
    }
}

pub(crate) fn arguments<'a, C, H: ProjectionHost<C>>(
    step: &'a ExpressionStep<H::Stage, H::Emitter, H::SetOperator>,
    host: &H,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<Vec<&'a ValueExpression<NativeExpression<C, H>>>, QueryError> {
    let mut values = Vec::new();
    let mut add_guard = |predicate: &'a Predicate<H::Stage>, budget: &mut Budget| {
        crate::eval::validate_predicate_leaves(predicate, budget, |guard, budget, range| {
            let arguments = host.value_arguments(guard, budget, range)?;
            crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                arguments.len(),
                budget,
                range,
            )?;
            values.extend(arguments.into_iter().map(|(value, _)| value));
            Ok(())
        })
    };
    match step {
        ExpressionStep::Filter(predicate) => add_guard(predicate, budget)?,
        ExpressionStep::Paths(paths) => add_guard(&paths.target, budget)?,
        ExpressionStep::Match(matching) => {
            for clause in &matching.clauses {
                match &clause.value {
                    MatchClause::From { source, .. } | MatchClause::Bind { value: source, .. } => {
                        crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                            1, budget, range,
                        )?;
                        values.push(source.value());
                    }
                    MatchClause::Where { predicate } => {
                        crate::eval::validate_predicate_leaves(
                            predicate,
                            budget,
                            |guard, budget, range| {
                                let arguments = host.value_arguments(guard, budget, range)?;
                                crate::projection::admit_slots::<
                                    &ValueExpression<NativeExpression<C, H>>,
                                >(arguments.len(), budget, range)?;
                                values.extend(arguments.into_iter().map(|(value, _)| value));
                                Ok(())
                            },
                        )?;
                    }
                }
            }
        }
        ExpressionStep::Ordering(ordering) => {
            if let Some(body) = &ordering.selector {
                crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                    1, budget, range,
                )?;
                values.push(body.value());
            }
        }
        ExpressionStep::Mapping(mapping) => {
            crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                2, budget, range,
            )?;
            values.push(mapping.key.value());
            values.push(mapping.value.value());
        }
        ExpressionStep::Group(body)
        | ExpressionStep::WhereExists(body)
        | ExpressionStep::FixedPoint(body) => {
            crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                1, budget, range,
            )?;
            values.push(body.value())
        }
        ExpressionStep::Head(value) | ExpressionStep::Tail(value) => {
            crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                1, budget, range,
            )?;
            values.push(value);
        }
        ExpressionStep::Check { operand, message } => {
            for value in operand.iter().chain(message.iter()) {
                crate::projection::admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
                    1, budget, range,
                )?;
                values.push(value);
            }
        }
        _ => {}
    }
    Ok(values)
}

type KindReader<'a, C, H> = dyn FnMut(
        &ValueExpression<NativeExpression<C, H>>,
        &BTreeMap<String, <H as ProjectionHost<C>>::Kind>,
        Option<<H as ProjectionHost<C>>::Kind>,
        bool,
        &mut Budget,
    ) -> Result<<H as ProjectionHost<C>>::Kind, QueryError>
    + 'a;

fn collection_argument_kind<C, H: ProjectionHost<C>>(
    value: &ValueExpression<NativeExpression<C, H>>,
    scalar: ScalarType,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    value_kind: &mut KindReader<'_, C, H>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let relative = host.collection_argument_kind(input, budget, &value.range)?;
    let kind = value_kind(value, scope, Some(relative), false, budget)?;
    let types = host
        .scalar_types(&kind, budget, &value.range)?
        .ok_or_else(|| {
            QueryError::query(value.range.clone(), "collection argument must be scalar")
        })?;
    for ty in &types {
        admit_scalar_type(
            ty,
            scalar,
            TypeFactPolicy::Deferred,
            budget,
            &Location::Query(value.range.clone()),
        )?;
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Native stage flow retains the active input, lexical scope, value reader and effect policy"
)]
pub(crate) fn kind<'q, C, H: ProjectionHost<C>>(
    step: &'q ExpressionStep<H::Stage, H::Emitter, H::SetOperator>,
    host: &H,
    input: Option<&H::Kind>,
    scope: &BTreeMap<String, H::Kind>,
    value_kind: &mut KindReader<'_, C, H>,
    allow_effects: bool,
    mut arguments: Option<&mut crate::ScopedNativeValueArguments<'q, C, H>>,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<H::Kind, QueryError> {
    let placeholder;
    let input = match input {
        Some(kind) => kind,
        None if explicit_source(step) => {
            placeholder = host.constructed_kind(vec![ValueType::Unestablished]);
            &placeholder
        }
        None => {
            return Err(QueryError::query(
                range.clone(),
                "functions require explicit native inputs",
            ));
        }
    };
    match step {
        ExpressionStep::Group(body) => value_kind(
            body.value(),
            scope,
            Some(input.clone()),
            allow_effects,
            budget,
        ),
        ExpressionStep::Filter(predicate) => {
            let subject = host.subject_kind(input, budget, range)?;
            guard_arguments(predicate, host, &subject, scope, value_kind, budget)?;
            Ok(input.clone())
        }
        ExpressionStep::Match(matching) => {
            let mut local = crate::projection::copy_scope::<C, H>(scope, budget, range)?;
            let mut captures: Vec<(Located<String>, H::Kind)> = Vec::new();
            for clause in &matching.clauses {
                match &clause.value {
                    MatchClause::From { name, source }
                    | MatchClause::Bind {
                        name,
                        value: source,
                    } => {
                        if name.value.is_empty()
                            || name.value == "input"
                            || local.contains_key(&name.value)
                        {
                            return Err(QueryError::query(
                                name.range.clone(),
                                "capture name is empty, reserved or already defined",
                            ));
                        }
                        let kind =
                            value_kind(source.value(), &local, Some(input.clone()), false, budget)?;
                        let kind = host.subject_kind(&kind, budget, &source.value().range)?;
                        budget.charge(
                            name.value.len() * 2 + 1,
                            Location::Query(name.range.clone()),
                        )?;
                        crate::projection::admit_slots::<(Located<String>, H::Kind)>(
                            1, budget, range,
                        )?;
                        if let Some(arguments) = arguments.as_deref_mut() {
                            crate::projection::admit_slots::<
                                crate::ScopedNativeArgument<'_, NativeExpression<C, H>, H::Kind>,
                            >(1, budget, range)?;
                            crate::projection::admit_slots::<(Located<String>, H::Kind)>(
                                captures.len(),
                                budget,
                                range,
                            )?;
                            for (name, _) in &captures {
                                budget.charge(
                                    name.value.len(),
                                    Location::Query(name.range.clone()),
                                )?;
                            }
                            arguments.push(crate::ScopedNativeArgument {
                                expression: source.value(),
                                relative: Some(input.clone()),
                                locals: captures.clone(),
                                allow_effects: false,
                            });
                        }
                        captures.push((name.clone(), kind.clone()));
                        local.insert(name.value.clone(), kind);
                    }
                    MatchClause::Where { predicate } => {
                        guard_arguments(predicate, host, input, &local, value_kind, budget)?;
                        if let Some(arguments) = arguments.as_deref_mut() {
                            append_guard_arguments(
                                predicate, host, input, &local, &captures, arguments, budget,
                            )?;
                        }
                    }
                }
            }
            crate::projection::admit_slots::<(String, H::Kind)>(captures.len(), budget, range)?;
            let fields = captures
                .into_iter()
                .map(|(name, kind)| (name.value, kind))
                .collect();
            host.capture_selection_kind(NativeCapture { fields }, budget, range)
        }
        ExpressionStep::Ordering(ordering) => {
            if ordering.selector.is_none() && host.is_map_kind(input) {
                return Ok(input.clone());
            }
            let subject = host.subject_kind(input, budget, range)?;
            let key = match &ordering.selector {
                Some(body) => value_kind(body.value(), scope, Some(subject), false, budget)?,
                None => subject,
            };
            host.ordering_key_kind(&key, budget, range)?;
            Ok(input.clone())
        }
        ExpressionStep::Mapping(mapping) => {
            let subject = host.subject_kind(input, budget, range)?;
            let key = value_kind(
                mapping.key.value(),
                scope,
                Some(subject.clone()),
                false,
                budget,
            )?;
            let value = value_kind(mapping.value.value(), scope, Some(subject), false, budget)?;
            host.mapping_kind(&key, &value, budget, range)
        }
        ExpressionStep::MapSorting => host.map_sort_kind(input, budget, range),
        ExpressionStep::Collection(operation) => {
            if matches!(operation, Stage::Filter(_) | Stage::Host(_)) {
                return Err(QueryError::query(
                    range.clone(),
                    "unsupported common collection tag",
                ));
            }
            if host.is_constructed_kind(input) {
                let types = host.scalar_types(input, budget, range)?.ok_or_else(|| {
                    QueryError::query(range.clone(), "constructed collection type is unavailable")
                })?;
                return Ok(host
                    .constructed_kind(collection_value_types(operation, &types, budget, range)?));
            }
            host.collection_kind(operation, input, budget, range)
        }
        ExpressionStep::Head(value) | ExpressionStep::Tail(value) => {
            collection_argument_kind(
                value,
                ScalarType::Integer,
                host,
                input,
                scope,
                value_kind,
                budget,
            )?;
            let operation = if matches!(step, ExpressionStep::Head(_)) {
                Stage::Head(0)
            } else {
                Stage::Tail(0)
            };
            if host.is_constructed_kind(input) {
                Ok(input.clone())
            } else {
                host.collection_kind(&operation, input, budget, range)
            }
        }
        ExpressionStep::Check { operand, message } => {
            if let Some(value) = operand {
                let relative = host.collection_argument_kind(input, budget, range)?;
                let kind = value_kind(value, scope, Some(relative), false, budget)?;
                let types = host.scalar_types(&kind, budget, range)?.ok_or_else(|| {
                    QueryError::query(
                        value.range.clone(),
                        "check operand must be an integer or string",
                    )
                })?;
                for ty in &types {
                    let scalar = if message.is_none()
                        && matches!(
                            ty,
                            ValueType::String
                                | ValueType::Declared {
                                    declaration: DeclaredType::String,
                                    ..
                                }
                        ) {
                        ScalarType::String
                    } else {
                        ScalarType::Integer
                    };
                    admit_scalar_type(
                        ty,
                        scalar,
                        TypeFactPolicy::Deferred,
                        budget,
                        &Location::Query(value.range.clone()),
                    )?;
                }
            }
            if let Some(value) = message {
                collection_argument_kind(
                    value,
                    ScalarType::String,
                    host,
                    input,
                    scope,
                    value_kind,
                    budget,
                )?;
            }
            if host.is_constructed_kind(input) {
                Ok(input.clone())
            } else {
                host.collection_kind(
                    &Stage::Check {
                        expected: None,
                        message: None,
                    },
                    input,
                    budget,
                    range,
                )
            }
        }
        ExpressionStep::WhereExists(body) => {
            let output = host.selection_kind(input, budget, range)?;
            let subject = host.subject_kind(input, budget, range)?;
            let result = value_kind(body.value(), scope, Some(subject), false, budget)?;
            host.selection_kind(&result, budget, range)?;
            host.validate_existence(input, &result, budget, range)?;
            Ok(output)
        }
        ExpressionStep::FixedPoint(body) => {
            host.selection_kind(input, budget, range)?;
            let emissions = allow_effects && host.allows_fixed_point_emission();
            if emissions {
                validate_fixed_point_leaves(body.query(), host, budget)?;
            }
            let result = value_kind(body.value(), scope, Some(input.clone()), emissions, budget)?;
            host.selection_kind(&result, budget, range)?;
            host.fixed_point_kind(input, &result, budget, range)
        }
        ExpressionStep::Paths(paths) => {
            let subject = host.subject_kind(input, budget, range)?;
            if !host.is_pure(&paths.edge.value) {
                return Err(QueryError::query(
                    paths.edge.range.clone(),
                    "path edges must be pure",
                ));
            }
            host.validate_path_edge(
                &paths.edge.value,
                &subject,
                scope,
                budget,
                &paths.edge.range,
            )?;
            guard_arguments(&paths.target, host, &subject, scope, value_kind, budget)?;
            let fields = ["source", "sink", "steps"]
                .into_iter()
                .map(|name| (name.to_owned(), subject.clone()))
                .collect();
            host.capture_selection_kind(NativeCapture { fields }, budget, range)
        }
        _ => unreachable!("common composition step"),
    }
}
fn guard_arguments<C, H: ProjectionHost<C>>(
    predicate: &Predicate<H::Stage>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    value_kind: &mut KindReader<'_, C, H>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    crate::eval::validate_predicate_leaves(predicate, budget, |guard, budget, range| {
        host.validate_guard_leaf(guard, input, scope, budget, range)?;
        for argument in host.scoped_value_arguments(guard, input, scope, budget, range)? {
            let mut local = crate::projection::copy_scope::<C, H>(scope, budget, range)?;
            for (name, kind) in argument.locals {
                local.insert(name.value, kind);
            }
            value_kind(
                argument.expression,
                &local,
                argument.relative,
                false,
                budget,
            )?;
        }
        Ok(())
    })
}

pub(crate) fn run<C, H: ProjectionHost<C>>(
    step: &ExpressionStep<H::Stage, H::Emitter, H::SetOperator>,
    host: &H,
    input: H::Value,
    mut arguments: &mut dyn NativeArguments<C, H>,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<H::Value, EvaluationError<H::Error>> {
    match step {
        ExpressionStep::Group(body) => arguments.evaluate_body(body, Some(input), None, budget),
        ExpressionStep::Filter(predicate) => {
            let items = host.selection_items(&input, arguments.parts().0, budget, range)?;
            let bindings =
                arguments
                    .parts()
                    .1
                    .with_shared_values(BTreeMap::new(), budget, range)?;
            let mut selected = Vec::new();
            crate::projection::admit_slots::<H::Value>(items.len(), budget, range)?;
            for item in items {
                let truth = crate::eval::evaluate_predicate(
                    predicate,
                    crate::Nesting::Nodes,
                    budget,
                    |guard, budget, range| {
                        host.evaluate_guard_leaf_with_arguments(
                            guard, &item, &bindings, arguments, budget, range,
                        )
                    },
                )?;
                if host.filter_truth(truth, arguments.parts().0, budget, &predicate.range)? {
                    selected.push(item);
                }
            }
            host.restore_selection(
                input,
                selected,
                SelectionShape::Filter,
                arguments.parts().0,
                budget,
                range,
            )
        }
        ExpressionStep::Ordering(ordering) => {
            if ordering.order == NativeOrder::Sort && ordering.selector.is_none() {
                if let Some(value) = host.pattern_constructed(&input) {
                    let value = crate::value::sort_value(
                        copy_constructed_value(value, budget, Location::Query(range.clone()))?,
                        None,
                        false,
                        budget,
                        &Location::Query(range.clone()),
                    )?;
                    let types = vec![constructed_value_type(
                        &value,
                        budget,
                        Location::Query(range.clone()),
                    )?];
                    return host.lift_constructed(value, types, arguments.parts().0, budget, range);
                }
                if host.is_map_kind(&host.kind(&input, budget, range)?) {
                    let entries = host.map_entries(
                        host.copy_value(&input, arguments.parts().0, budget, range)?,
                        arguments.parts().0,
                        budget,
                        range,
                    )?;
                    let entries = order_native_by(
                        entries,
                        NativeOrder::Sort,
                        budget,
                        Location::Query(range.clone()),
                        |(key, _), budget| {
                            budget.charge(key.len(), Location::Query(range.clone()))?;
                            budget.charge_results(key.len(), Location::Query(range.clone()))?;
                            Ok(NativeOrderKey::Text(key.clone()))
                        },
                    )?;
                    return host.restore_map(input, entries, arguments.parts().0, budget, range);
                }
            }
            host.admit_ordering(&input, ordering.order, arguments.parts().0, budget, range)?;
            let items = host.selection_items(&input, arguments.parts().0, budget, range)?;
            let items = order_native_by(
                items,
                ordering.order,
                budget,
                Location::Query(range.clone()),
                |item, budget| {
                    let relative = host.copy_value(item, arguments.parts().0, budget, range)?;
                    let key = match &ordering.selector {
                        Some(body) => {
                            arguments.evaluate_body(body, Some(relative), None, budget)?
                        }
                        None => relative,
                    };
                    host.ordering_key(key, arguments.parts().0, budget, range)
                },
            )?;
            host.restore_selection(
                input,
                items,
                SelectionShape::Preserve,
                arguments.parts().0,
                budget,
                range,
            )
        }
        ExpressionStep::Mapping(mapping) => {
            let items = host.selection_items(&input, arguments.parts().0, budget, range)?;
            let entries = map_native_by(
                items,
                &mut arguments,
                budget,
                Location::Query(range.clone()),
                |item, arguments, budget| {
                    let input = host.copy_value(item, arguments.parts().0, budget, range)?;
                    let key = arguments.evaluate_body(&mapping.key, Some(input), None, budget)?;
                    host.mapping_keys(key, arguments.parts().0, budget, range)
                },
                |item, arguments, budget| {
                    arguments.evaluate_body(&mapping.value, Some(item), None, budget)
                },
            )?;
            host.construct_map(entries, arguments.parts().0, budget, range)
        }
        ExpressionStep::MapSorting => {
            let copied = host.copy_value(&input, arguments.parts().0, budget, range)?;
            let entries = host.map_entries(copied, arguments.parts().0, budget, range)?;
            let entries = sort_map_by_value(
                entries,
                budget,
                Location::Query(range.clone()),
                |value, budget| {
                    let value = host.copy_value(value, arguments.parts().0, budget, range)?;
                    scalar_map_key(
                        host.ordering_key(value, arguments.parts().0, budget, range)?,
                        range,
                    )
                },
            )?;
            host.restore_map(input, entries, arguments.parts().0, budget, range)
        }
        ExpressionStep::Collection(operation) => {
            run_collection(operation, host, input, arguments.parts().0, budget, range)
        }
        ExpressionStep::Head(_) | ExpressionStep::Tail(_) | ExpressionStep::Check { .. } => {
            let argument = match step {
                ExpressionStep::Head(value) | ExpressionStep::Tail(value) => Some(value),
                ExpressionStep::Check { operand, .. } => operand.as_ref(),
                _ => unreachable!(),
            };
            let mut count = None;
            let mut text = None;
            if let Some(expression) = argument {
                let relative =
                    host.collection_argument_input(&input, arguments.parts().0, budget, range)?;
                let value = arguments
                    .evaluate(expression, relative, budget)?
                    .into_owned(budget)?
                    .into_data();
                match value {
                    ConstructedValue::Integer(value) => {
                        count = Some(nonnegative_integer_to_usize(
                            value,
                            Location::Query(expression.range.clone()),
                            "collection bound must be nonnegative",
                        )?)
                    }
                    ConstructedValue::String(value)
                        if matches!(step, ExpressionStep::Check { message: None, .. }) =>
                    {
                        text = Some(value)
                    }
                    _ => {
                        return Err(QueryError::query(
                            expression.range.clone(),
                            "collection bound must be an integer, or a check message string",
                        )
                        .into());
                    }
                }
            }
            let operation = match step {
                ExpressionStep::Head(_) => Stage::Head(count.expect("head count")),
                ExpressionStep::Tail(_) => Stage::Tail(count.expect("tail count")),
                ExpressionStep::Check { message, .. } => {
                    if let Some(expression) = message {
                        let relative = host.collection_argument_input(
                            &input,
                            arguments.parts().0,
                            budget,
                            range,
                        )?;
                        let value = arguments
                            .evaluate(expression, relative, budget)?
                            .into_owned(budget)?
                            .into_data();
                        let ConstructedValue::String(value) = value else {
                            return Err(QueryError::query(
                                expression.range.clone(),
                                "check message must be a string",
                            )
                            .into());
                        };
                        text = Some(value);
                    }
                    Stage::Check {
                        expected: count,
                        message: text,
                    }
                }
                _ => unreachable!(),
            };
            run_collection(&operation, host, input, arguments.parts().0, budget, range)
        }
        ExpressionStep::WhereExists(body) => {
            let input = host.admit_selection(input, arguments.parts().0, budget, range)?;
            let items = host.selection_items(&input, arguments.parts().0, budget, range)?;
            let items = filter_native_exists(
                items,
                budget,
                Location::Query(range.clone()),
                |item, budget| {
                    let relative = host.copy_value(item, arguments.parts().0, budget, range)?;
                    let relative =
                        host.admit_selection(relative, arguments.parts().0, budget, range)?;
                    let value = arguments.evaluate_body(body, Some(relative), None, budget)?;
                    let value = host.admit_selection(value, arguments.parts().0, budget, range)?;
                    host.selection_items(&value, arguments.parts().0, budget, range)
                },
            )?;
            host.restore_selection(
                input,
                items,
                SelectionShape::Filter,
                arguments.parts().0,
                budget,
                range,
            )
        }
        ExpressionStep::FixedPoint(body) => {
            let input = host.admit_selection(input, arguments.parts().0, budget, range)?;
            evaluate_native_fixed_point(
                input,
                &mut arguments,
                budget,
                Location::Query(range.clone()),
                |current, arguments, budget| {
                    let input = host.copy_value(current, arguments.parts().0, budget, range)?;
                    let input = host.admit_selection(input, arguments.parts().0, budget, range)?;
                    let result = arguments.evaluate_body(body, Some(input), None, budget)?;
                    host.admit_selection(result, arguments.parts().0, budget, range)
                },
                |value, arguments, budget| {
                    let value = host.copy_value(value, arguments.parts().0, budget, range)?;
                    host.admit_selection(value, arguments.parts().0, budget, range)
                },
                |left, right, arguments, budget| {
                    host.same_selection(left, right, arguments.parts().0, budget, range)
                },
            )
        }
        ExpressionStep::Match(matching) => {
            let schema =
                crate::matching::prepared_schema(matching, host, &input, arguments, budget)?;
            let rows = crate::matching::evaluate_prepared_match(
                matching, &schema, host, input, arguments, budget,
            )?;
            host.construct_captures(rows, schema, arguments.parts().0, budget, range)
        }
        ExpressionStep::Paths(paths) => {
            let kind = host.kind(&input, budget, range)?;
            let subject = host.subject_kind(&kind, budget, range)?;
            let schema = NativeCapture {
                fields: ["source", "sink", "steps"]
                    .into_iter()
                    .map(|name| (name.to_owned(), subject.clone()))
                    .collect(),
            };
            let rows = evaluate_paths_with_arguments(paths, host, input, arguments, budget)?;
            host.construct_captures(rows, schema, arguments.parts().0, budget, range)
        }
        _ => unreachable!("common composition step"),
    }
}

impl<S, E, O> ExpressionStep<S, E, O> {
    /// Visit direct common value bodies in written order. Native payloads stay opaque.
    pub fn for_each_value<'a>(
        &'a self,
        mut visit: impl FnMut(&'a ValueExpression<Expression<S, O, E>>),
    ) {
        match self {
            Self::Value(value) | Self::Head(value) | Self::Tail(value) => visit(value),
            Self::Check { operand, message } => {
                for value in operand.iter().chain(message.iter()) {
                    visit(value);
                }
            }
            Self::Projection(projection) => {
                for (_, value) in &projection.fields {
                    visit(value);
                }
            }
            Self::Match(matching) => {
                for clause in &matching.clauses {
                    match &clause.value {
                        MatchClause::From { source, .. }
                        | MatchClause::Bind { value: source, .. } => visit(source.value()),
                        _ => {}
                    }
                }
            }
            Self::Ordering(ordering) => {
                if let Some(body) = &ordering.selector {
                    visit(body.value());
                }
            }
            Self::Mapping(mapping) => {
                visit(mapping.key.value());
                visit(mapping.value.value());
            }
            Self::Group(body) | Self::WhereExists(body) | Self::FixedPoint(body) => {
                visit(body.value())
            }
            _ => {}
        }
    }
    /// Visit native stage, guard and edge payloads, preserving their written order.
    pub fn for_each_payload<'a>(&'a self, mut visit: impl FnMut(&'a S)) {
        fn leaves<'a, S>(predicate: &'a Predicate<S>, visit: &mut impl FnMut(&'a S)) {
            let mut pending = vec![predicate];
            while let Some(predicate) = pending.pop() {
                match &predicate.value {
                    PredicateKind::Leaf(leaf) => visit(leaf),
                    PredicateKind::Not(child) | PredicateKind::Group(child) => pending.push(child),
                    PredicateKind::And(children) | PredicateKind::Or(children) => {
                        pending.extend(children.iter().rev())
                    }
                }
            }
        }
        match self {
            Self::Stage(stage) => visit(stage),
            Self::Filter(predicate) => leaves(predicate, &mut visit),
            Self::Match(matching) => {
                for clause in &matching.clauses {
                    if let MatchClause::Where { predicate } = &clause.value {
                        leaves(predicate, &mut visit);
                    }
                }
            }
            Self::Paths(paths) => {
                visit(&paths.edge.value);
                leaves(&paths.target, &mut visit);
            }
            _ => {}
        }
    }
}

pub(crate) fn dispose_step<S, O, E>(
    step: ExpressionStep<S, E, O>,
    values: &mut Vec<ValueExpression<Expression<S, O, E>>>,
) {
    match step {
        ExpressionStep::Value(value)
        | ExpressionStep::Head(value)
        | ExpressionStep::Tail(value) => {
            if let Ok(value) = std::sync::Arc::try_unwrap(value) {
                values.push(value);
            }
        }
        ExpressionStep::Check { operand, message } => {
            for value in operand.into_iter().chain(message) {
                if let Ok(value) = std::sync::Arc::try_unwrap(value) {
                    values.push(value);
                }
            }
        }
        ExpressionStep::Projection(value) => {
            if let Ok(value) = std::sync::Arc::try_unwrap(value) {
                values.extend(value.fields.into_iter().map(|(_, value)| value));
            }
        }
        ExpressionStep::Filter(predicate) => {
            if let Ok(predicate) = std::sync::Arc::try_unwrap(predicate) {
                crate::matching::dispose_predicate(predicate);
            }
        }
        ExpressionStep::Match(matching) => {
            if let Ok(matching) = std::sync::Arc::try_unwrap(matching) {
                for clause in matching.clauses {
                    match clause.value {
                        MatchClause::From { source, .. }
                        | MatchClause::Bind { value: source, .. } => {
                            values.push(source.into_value())
                        }
                        MatchClause::Where { predicate } => {
                            crate::matching::dispose_predicate(predicate)
                        }
                    }
                }
            }
        }
        ExpressionStep::Ordering(ordering) => {
            if let Ok(ordering) = std::sync::Arc::try_unwrap(ordering)
                && let Some(body) = ordering.selector
            {
                values.push(body.into_value());
            }
        }
        ExpressionStep::Mapping(mapping) => {
            if let Ok(mapping) = std::sync::Arc::try_unwrap(mapping) {
                values.push(mapping.key.into_value());
                values.push(mapping.value.into_value());
            }
        }
        ExpressionStep::Group(body)
        | ExpressionStep::WhereExists(body)
        | ExpressionStep::FixedPoint(body) => {
            if let Ok(body) = std::sync::Arc::try_unwrap(body) {
                values.push(body.into_value());
            }
        }
        ExpressionStep::Paths(paths) => {
            if let Ok(paths) = std::sync::Arc::try_unwrap(paths) {
                crate::dispose_paths(paths);
            }
        }
        _ => {}
    }
}

pub(crate) fn scoped_arguments<'a, C, H: ProjectionHost<C>>(
    step: &'a ExpressionStep<H::Stage, H::Emitter, H::SetOperator>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    allow_effects: bool,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<crate::ScopedNativeValueArguments<'a, C, H>, QueryError> {
    let mut result = Vec::new();
    let mut add_body = |body: &'a NativeBody<NativeExpression<C, H>>,
                        relative: H::Kind,
                        locals: Vec<(Located<String>, H::Kind)>,
                        allow_effects: bool,
                        budget: &mut Budget| {
        crate::projection::admit_slots::<
            crate::ScopedNativeArgument<'_, NativeExpression<C, H>, H::Kind>,
        >(1, budget, range)?;
        result.push(crate::ScopedNativeArgument {
            expression: body.value(),
            relative: Some(relative),
            locals,
            allow_effects,
        });
        Ok::<_, QueryError>(())
    };
    match step {
        ExpressionStep::Group(body) => {
            add_body(body, input.clone(), Vec::new(), allow_effects, budget)?
        }
        ExpressionStep::Ordering(ordering) => {
            if let Some(body) = &ordering.selector {
                add_body(
                    body,
                    host.subject_kind(input, budget, range)?,
                    Vec::new(),
                    false,
                    budget,
                )?;
            }
        }
        ExpressionStep::Mapping(mapping) => {
            let subject = host.subject_kind(input, budget, range)?;
            add_body(&mapping.key, subject.clone(), Vec::new(), false, budget)?;
            add_body(&mapping.value, subject, Vec::new(), false, budget)?;
        }
        ExpressionStep::WhereExists(body) => add_body(
            body,
            host.subject_kind(input, budget, range)?,
            Vec::new(),
            false,
            budget,
        )?,
        ExpressionStep::FixedPoint(body) => add_body(
            body,
            input.clone(),
            Vec::new(),
            allow_effects && host.allows_fixed_point_emission(),
            budget,
        )?,
        _ => {}
    }
    match step {
        ExpressionStep::Head(value) | ExpressionStep::Tail(value) => {
            crate::projection::admit_slots::<
                crate::ScopedNativeArgument<'_, NativeExpression<C, H>, H::Kind>,
            >(1, budget, range)?;
            result.push(crate::ScopedNativeArgument {
                expression: value,
                relative: Some(host.collection_argument_kind(input, budget, range)?),
                locals: Vec::new(),
                allow_effects: false,
            })
        }
        ExpressionStep::Check { operand, message } => {
            for value in operand.iter().chain(message.iter()) {
                crate::projection::admit_slots::<
                    crate::ScopedNativeArgument<'_, NativeExpression<C, H>, H::Kind>,
                >(1, budget, range)?;
                result.push(crate::ScopedNativeArgument {
                    expression: value,
                    relative: Some(host.collection_argument_kind(input, budget, range)?),
                    locals: Vec::new(),
                    allow_effects: false,
                });
            }
        }
        ExpressionStep::Filter(predicate) => append_guard_arguments(
            predicate,
            host,
            &host.subject_kind(input, budget, range)?,
            scope,
            &[],
            &mut result,
            budget,
        )?,
        ExpressionStep::Paths(paths) => append_guard_arguments(
            &paths.target,
            host,
            &host.subject_kind(input, budget, range)?,
            scope,
            &[],
            &mut result,
            budget,
        )?,
        _ => {}
    }
    Ok(result)
}
fn append_guard_arguments<'a, C, H: ProjectionHost<C>>(
    predicate: &'a Predicate<H::Stage>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    captures: &[(Located<String>, H::Kind)],
    result: &mut crate::ScopedNativeValueArguments<'a, C, H>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    crate::eval::validate_predicate_leaves(predicate, budget, |guard, budget, range| {
        let arguments = host.scoped_value_arguments(guard, input, scope, budget, range)?;
        crate::projection::admit_slots::<
            crate::ScopedNativeArgument<'_, NativeExpression<C, H>, H::Kind>,
        >(arguments.len(), budget, range)?;
        for mut argument in arguments {
            crate::projection::admit_slots::<(Located<String>, H::Kind)>(
                captures.len(),
                budget,
                range,
            )?;
            for (name, _) in captures {
                budget.charge(name.value.len(), Location::Query(name.range.clone()))?;
            }
            argument.locals.splice(0..0, captures.iter().cloned());
            result.push(argument);
        }
        Ok(())
    })
}

/// Exact native identity or an explicitly projected text value for stable unique.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NativeCollectionKey {
    Native(NativeIdentity),
    Text(String),
    Named(String, NativeIdentity),
}

/// Common constructed type flow for native hosts' overloaded stream operations.
pub fn collection_value_types(
    operation: &Stage<()>,
    types: &[ValueType],
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<Vec<ValueType>, QueryError> {
    let input = Located::new(ValueExpressionKind::Name("input".to_owned()), range.clone());
    let expression: ValueExpression = Located::new(
        match operation {
            Stage::Count => ValueExpressionKind::Reduce {
                operator: ValueReducer::Length,
                array: Box::new(input),
            },
            Stage::Unique => ValueExpressionKind::Unique(Box::new(input)),
            Stage::Tally => ValueExpressionKind::Tally(Box::new(input)),
            Stage::Head(_) | Stage::Tail(_) | Stage::Reverse | Stage::Check { .. } => {
                return copy_value_types(types, budget, Location::Query(range.clone()));
            }
            _ => {
                return Err(QueryError::query(
                    range.clone(),
                    "unsupported common collection tag",
                ));
            }
        },
        range.clone(),
    );
    let facts = BTreeMap::from([(
        "input".to_owned(),
        copy_value_types(types, budget, Location::Query(range.clone()))?,
    )]);
    crate::query_value_types(
        &expression,
        &facts,
        &mut crate::value::NoQueries { bindings: None },
        budget,
    )
}
struct Collections<'h, H>(&'h H);
impl<C, H: ProjectionHost<C>> Host<H::Value, &mut C, ()> for Collections<'_, H> {
    type Error = H::Error;
    type Key = NativeCollectionKey;
    fn prepare_stage(
        &self,
        _stage: &Located<Stage<()>>,
        _context: &mut &mut C,
        _budget: &mut Budget,
    ) -> Result<(), EvaluationError<Self::Error>> {
        Ok(())
    }
    fn evaluate_leaf(
        &self,
        _leaf: &(),
        _item: &H::Value,
        _context: &mut &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<Self::Error>> {
        Err(QueryError::query(
            range.clone(),
            "collection filters use direct core predicates",
        )
        .into())
    }
    fn identity_key(
        &self,
        item: &H::Value,
        context: &mut &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Key, EvaluationError<Self::Error>> {
        self.0.collection_identity(item, context, budget, range)
    }
    fn tally_key(
        &self,
        item: H::Value,
        context: &mut &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<String, EvaluationError<Self::Error>> {
        self.0.collection_tally_key(item, context, budget, range)
    }
}
fn run_collection<C, H: ProjectionHost<C>>(
    operation: &Stage<()>,
    host: &H,
    input: H::Value,
    context: &mut C,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<H::Value, EvaluationError<H::Error>> {
    if let Some(value) = host.pattern_constructed(&input) {
        let location = Location::Query(range.clone());
        let value = match operation {
            Stage::Count => Some(crate::value::reduce_ref(
                ValueReducer::Length,
                value,
                budget,
                &location,
            )?),
            Stage::Unique => Some(crate::value::unique_value(
                copy_constructed_value(value, budget, location.clone())?,
                budget,
                &location,
            )?),
            Stage::Tally => Some(crate::value::tally_value(
                copy_constructed_value(value, budget, location.clone())?,
                false,
                None,
                budget,
                &location,
            )?),
            _ => None,
        };
        if let Some(value) = value {
            let types = vec![constructed_value_type(&value, budget, location)?];
            return host.lift_constructed(value, types, context, budget, range);
        }
    }
    if host.is_map_kind(&host.kind(&input, budget, range)?)
        && matches!(operation, Stage::Head(_) | Stage::Tail(_) | Stage::Reverse)
    {
        let value = host.copy_value(&input, context, budget, range)?;
        let mut entries = host.map_entries(value, context, budget, range)?;
        let selection = match operation {
            Stage::Head(n) => MapSelection::Head(*n),
            Stage::Tail(n) => MapSelection::Tail(*n),
            _ => MapSelection::Reverse,
        };
        select_map(
            &mut entries,
            selection,
            budget,
            Location::Query(range.clone()),
        )?;
        return host.restore_map(input, entries, context, budget, range);
    }
    if matches!(operation, Stage::Filter(_) | Stage::Host(_)) {
        return Err(QueryError::query(range.clone(), "unsupported common collection tag").into());
    }
    let items = host.selection_items(&input, context, budget, range)?;
    let pipeline = Pipeline::new(
        vec![Located::new(operation.clone(), range.clone())],
        range.clone(),
        budget.remaining_nesting(),
    )?;
    let admitted =
        NativeInput::from_admitted(context, items, budget, Location::Query(range.clone()))?;
    let result = evaluate(&pipeline, &Collections(host), admitted, budget)?;
    host.restore_collection(input, result.value, result.context, budget, range)
}

pub(crate) fn ends_constructed_tally<S, O, E>(expression: &Expression<S, O, E>) -> bool {
    match &expression.value {
        ExpressionKind::Group(value) => ends_constructed_tally(value),
        ExpressionKind::Sequence { input, steps } => steps.last().map_or_else(
            || ends_constructed_tally(input),
            |step| matches!(step.value, ExpressionStep::Collection(Stage::Tally)),
        ),
        _ => false,
    }
}

fn scalar_map_key<E>(
    key: NativeOrderKey,
    range: &Range<usize>,
) -> Result<ScalarMapKey, EvaluationError<E>> {
    Ok(match key {
        NativeOrderKey::Scalar(value) => value,
        NativeOrderKey::Count(value) => ScalarMapKey::Unsigned(value as u64),
        NativeOrderKey::Text(value) => ScalarMapKey::String(value),
        NativeOrderKey::Value(value) => match value.into_data() {
            ConstructedValue::Absent => ScalarMapKey::Null,
            ConstructedValue::Boolean(value) => ScalarMapKey::Boolean(value),
            value @ (ConstructedValue::Integer(_)
            | ConstructedValue::Float(_)
            | ConstructedValue::String(_)
            | ConstructedValue::Member(_)) => ScalarMapKey::Typed(value),
            _ => {
                return Err(QueryError::query(
                    range.clone(),
                    "sort_by(value) requires scalar map values",
                )
                .into());
            }
        },
    })
}

fn validate_fixed_point_leaves<C, H: ProjectionHost<C>>(
    expression: &NativeExpression<C, H>,
    host: &H,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    crate::walk_expression(expression, budget, MAX_NESTING, |expression, budget| {
        if let ExpressionKind::Sequence { steps, .. } = &expression.value {
            for step in steps {
                budget.charge(1, Location::Query(step.range.clone()))?;
                if let ExpressionStep::Stage(stage) = &step.value
                    && !host.is_pure(stage)
                {
                    return Err(QueryError::query(
                        step.range.clone(),
                        "fixpoint bodies cannot edit",
                    ));
                }
            }
        }
        Ok(())
    })
}
