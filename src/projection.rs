use std::{collections::BTreeMap, ops::Range};

use crate::{
    ArrayType, Bindings, Budget, ConstructedValue, EvaluationError, Expression, ExpressionKind,
    ExpressionStep, Located, Location, ProgramHost, QueryError, QueryErrorCategory,
    ValueExpression, ValueQueryHost, ValueType, copy_constructed_value, copy_value_types,
    evaluate_expression, evaluate_query_value, query_value_types,
};

/// Correlated native subjects become positional records with written field order.
pub type ProjectionFields<Q> = Vec<(Located<String>, ValueExpression<Q>)>;
pub type NativeExpression<C, H> = Expression<
    <H as ProgramHost<C>>::Stage,
    <H as ProgramHost<C>>::SetOperator,
    <H as ProgramHost<C>>::Emitter,
>;

#[derive(Debug, PartialEq)]
pub struct Projection<Q> {
    pub subject: Located<String>,
    pub fields: ProjectionFields<Q>,
}

pub type NativeOperands<C, H> = Vec<(NativeExpression<C, H>, <H as ProjectionHost<C>>::Kind)>;

/// Domain facts and explicit scalar conversions for shared native projection.
/// Hooks describe native stages; the core owns query composition and pure scope.
pub trait ProjectionHost<C>: ProgramHost<C> {
    type Kind: Clone + PartialEq;

    fn kind(
        &self,
        value: &Self::Value,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Kind, QueryError>;
    fn scalar_types(
        &self,
        kind: &Self::Kind,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Option<Vec<ValueType>>, QueryError>;
    fn constructed_kind(&self, types: Vec<ValueType>) -> Self::Kind;
    fn stage_kind(
        &self,
        stage: &Self::Stage,
        input: &Self::Kind,
        scope: &BTreeMap<String, Self::Kind>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Kind, QueryError>;
    fn is_pure(&self, stage: &Self::Stage) -> bool;
    /// Expose domain selector operands as normalized expressions with their relative kind.
    fn nested_queries(
        &self,
        stage: &Self::Stage,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<NativeOperands<C, Self>, QueryError>;
    fn subjects(
        &self,
        input: Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<Self::Value>, EvaluationError<Self::Error>>;
    /// Declared singleton kind yielded by subjects, checked even for empty input.
    /// The default preserves hosts whose stream and singleton share one kind.
    fn subject_kind(
        &self,
        input: &Self::Kind,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Self::Kind, QueryError> {
        Ok(input.clone())
    }
    /// Borrow the declared capture fields, including an empty collection schema.
    fn capture_record_kind<'k>(
        &self,
        _kind: &'k Self::Kind,
    ) -> Option<&'k crate::NativeCapture<Self::Kind>> {
        None
    }
    fn project_value(
        &self,
        value: Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<ConstructedValue, EvaluationError<Self::Error>>;
    fn lift_constructed(
        &self,
        value: ConstructedValue,
        types: Vec<ValueType>,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>;
}

/// Preflight a pure normalized query against declared domain kinds, including
/// nested native operands. It executes no document operation.
///
/// # Errors
/// Undefined bindings, emitters, effects, transitions and budgets fail at query ranges.
pub fn native_expression_kind<C, H: ProjectionHost<C>>(
    expression: &Expression<H::Stage, H::SetOperator, H::Emitter>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<H::Kind, QueryError> {
    let location = Location::Query(expression.range.clone());
    budget.within_evaluation(expression.value.depth(), location.clone(), |budget| {
        budget.charge(1, location)?;
        match &expression.value {
            ExpressionKind::Input => Ok(input.clone()),
            ExpressionKind::Binding(name) => scope.get(name).cloned().ok_or_else(|| {
                QueryError::query(
                    expression.range.clone(),
                    format!("undefined binding: {name}"),
                )
            }),
            ExpressionKind::Group(child) => {
                native_expression_kind(child, host, input, scope, budget)
            }
            ExpressionKind::Field { value, field } => {
                let kind = native_expression_kind(value, host, input, scope, budget)?;
                let record = host.capture_record_kind(&kind).ok_or_else(|| {
                    QueryError::query(
                        field.range.clone(),
                        "field access requires a native capture",
                    )
                })?;
                Ok(record.get(field, budget)?.clone())
            }
            ExpressionKind::Sequence {
                input: source,
                steps,
            } => {
                let mut kind = native_expression_kind(source, host, input, scope, budget)?;
                for step in steps {
                    budget.charge(1, Location::Query(step.range.clone()))?;
                    let ExpressionStep::Stage(stage) = &step.value else {
                        return Err(QueryError::query(
                            step.range.clone(),
                            "constructed expressions cannot emit",
                        ));
                    };
                    if !host.is_pure(stage) {
                        return Err(QueryError::query(
                            step.range.clone(),
                            "constructed expressions cannot edit or emit",
                        ));
                    }
                    for (query, relative) in host.nested_queries(stage, budget, &step.range)? {
                        let result = native_expression_kind(&query, host, &relative, scope, budget);
                        crate::dispose_expression(query);
                        result?;
                    }
                    kind = host.stage_kind(stage, &kind, scope, budget, &step.range)?;
                }
                Ok(kind)
            }
            ExpressionKind::Set { .. } => {
                let mut pending = Vec::new();
                let mut operand = expression;
                while let ExpressionKind::Set { left, right, .. } = &operand.value {
                    admit_slots::<(
                        &Expression<H::Stage, H::SetOperator, H::Emitter>,
                        &Range<usize>,
                    )>(1, budget, &operand.range)?;
                    pending.push((right.as_ref(), &operand.range));
                    operand = left;
                }
                let left = native_expression_kind(operand, host, input, scope, budget)?;
                while let Some((right, range)) = pending.pop() {
                    let right = native_expression_kind(right, host, input, scope, budget)?;
                    if left != right {
                        return Err(QueryError::query(
                            range.clone(),
                            "set operands require the same native kind",
                        ));
                    }
                }
                Ok(left)
            }
            ExpressionKind::Record(_) => Err(QueryError::query(
                expression.range.clone(),
                "native records require explicit scalar field projections",
            )),
        }
    })
}

fn admit_slots<T>(len: usize, budget: &mut Budget, range: &Range<usize>) -> Result<(), QueryError> {
    if len
        .checked_mul(std::mem::size_of::<T>())
        .is_none_or(|size| size > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "projection allocation size overflow",
        ));
    }
    budget.charge(len, Location::Query(range.clone()))
}

pub(crate) fn copy_scope<C, H: ProjectionHost<C>>(
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<BTreeMap<String, H::Kind>, QueryError> {
    let bytes = scope.keys().try_fold(0usize, |bytes, name| {
        bytes
            .checked_add(name.len())
            .and_then(|size| size.checked_add(1))
            .ok_or_else(|| QueryError::query(range.clone(), "scope metadata size overflow"))
    })?;
    budget.charge(bytes, Location::Query(range.clone()))?;
    Ok(scope.clone())
}

pub(crate) fn scope_kinds<C, H: ProjectionHost<C>>(
    host: &H,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<BTreeMap<String, H::Kind>, QueryError> {
    let mut scope = BTreeMap::new();
    for (name, value) in bindings.iter() {
        budget.charge(name.len() + 1, Location::Query(range.clone()))?;
        scope.insert(name.to_owned(), host.kind(value, budget, range)?);
    }
    Ok(scope)
}

fn value_facts<C, H: ProjectionHost<C>>(
    host: &H,
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<BTreeMap<String, Vec<ValueType>>, QueryError> {
    let mut facts = BTreeMap::new();
    for (name, kind) in scope {
        budget.charge(name.len() + 1, Location::Query(range.clone()))?;
        // Native names are admitted for no-shadowing, but validate_name refuses
        // reading their handles as scalar data.
        let types = host
            .scalar_types(kind, budget, range)?
            .unwrap_or_else(|| vec![ValueType::Unestablished]);
        facts.insert(name.to_owned(), types);
    }
    Ok(facts)
}

struct NativeValues<'a, C, H: ProjectionHost<C>> {
    host: &'a H,
    input_kind: H::Kind,
    input: Option<&'a H::Value>,
    context: Option<&'a mut C>,
    bindings: Option<&'a Bindings<H::Value>>,
    scope: BTreeMap<String, H::Kind>,
}

impl<C, H: ProjectionHost<C>> ValueQueryHost<Expression<H::Stage, H::SetOperator, H::Emitter>>
    for NativeValues<'_, C, H>
{
    type Error = EvaluationError<H::Error>;
    fn query_types(
        &mut self,
        query: &Expression<H::Stage, H::SetOperator, H::Emitter>,
        locals: &[(&str, Vec<ValueType>)],
        budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        let mut scope = copy_scope::<C, H>(&self.scope, budget, &query.range)?;
        for (name, types) in locals {
            budget.charge(name.len() + 1, Location::Query(query.range.clone()))?;
            scope.insert(
                (*name).to_owned(),
                self.host.constructed_kind(copy_value_types(
                    types,
                    budget,
                    Location::Query(query.range.clone()),
                )?),
            );
        }
        let kind = native_expression_kind(query, self.host, &self.input_kind, &scope, budget)?;
        self.host
            .scalar_types(&kind, budget, &query.range)?
            .ok_or_else(|| {
                QueryError::query(
                    query.range.clone(),
                    "native handles require an explicit scalar projection",
                )
            })
    }
    fn evaluate_query(
        &mut self,
        query: &Expression<H::Stage, H::SetOperator, H::Emitter>,
        locals: &[(&str, ConstructedValue)],
        budget: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        let context = self.context.as_deref_mut().expect("runtime native context");
        let mut scoped = None;
        for (name, value) in locals {
            let value =
                copy_constructed_value(value, budget, Location::Query(query.range.clone()))?;
            admit_slots::<ValueType>(1, budget, &query.range)?;
            let types = vec![crate::constructed_value_type(
                &value,
                budget,
                Location::Query(query.range.clone()),
            )?];
            let value = self
                .host
                .lift_constructed(value, types, context, budget, &query.range)?;
            let outer = scoped
                .as_ref()
                .unwrap_or(self.bindings.expect("runtime native bindings"));
            budget.charge(name.len(), Location::Query(query.range.clone()))?;
            scoped = Some(outer.with_local(
                Located::new((*name).to_owned(), query.range.clone()),
                value,
                budget,
            )?);
        }
        let input = self.host.copy_value(
            self.input.expect("runtime relative input"),
            context,
            budget,
            &query.range,
        )?;
        let value = evaluate_expression(
            query,
            self.host,
            input,
            context,
            scoped
                .as_ref()
                .unwrap_or(self.bindings.expect("runtime native bindings")),
            budget,
        )?;
        self.host
            .project_value(value, context, budget, &query.range)
    }
    fn read_binding(
        &mut self,
        name: &str,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        let context = self.context.as_deref_mut().expect("runtime native context");
        let value = self.host.copy_value(
            self.bindings
                .expect("runtime native bindings")
                .get(name, range.clone())?,
            context,
            budget,
            range,
        )?;
        self.host.project_value(value, context, budget, range)
    }
    fn validate_name(
        &mut self,
        name: &str,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let kind = self.scope.get(name).ok_or_else(|| {
            QueryError::query(range.clone(), format!("undefined binding: {name}"))
        })?;
        self.host
            .scalar_types(kind, budget, range)?
            .ok_or_else(|| {
                QueryError::query(
                    range.clone(),
                    "native handles require an explicit scalar projection",
                )
            })?;
        Ok(())
    }
}

/// Infer constructed field schemas before obtaining any native subjects.
///
/// # Errors
/// Shadowing, duplicate labels, invalid types/scopes/effects and budgets fail even on empty input.
pub fn projection_types<C, H: ProjectionHost<C>>(
    projection: &Projection<Expression<H::Stage, H::SetOperator, H::Emitter>>,
    host: &H,
    input: H::Kind,
    mut scope: BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<Vec<ValueType>, QueryError> {
    let range = &projection.subject.range;
    if scope.contains_key(&projection.subject.value) {
        return Err(QueryError::query(
            range.clone(),
            "projection subject shadows an existing binding",
        ));
    }
    budget.charge(
        projection.subject.value.len() + 1,
        Location::Query(range.clone()),
    )?;
    let subject_kind = host.subject_kind(&input, budget, range)?;
    scope.insert(projection.subject.value.clone(), subject_kind.clone());
    let facts = value_facts(host, &scope, budget, range)?;
    let mut queries = NativeValues::<C, H> {
        host,
        input_kind: subject_kind,
        input: None,
        context: None,
        bindings: None,
        scope,
    };
    admit_slots::<Vec<ValueType>>(1, budget, range)?;
    let mut positions = vec![Vec::new()];
    for (index, (label, expression)) in projection.fields.iter().enumerate() {
        for (previous, _) in &projection.fields[..index] {
            budget.charge(
                label.value.len().min(previous.value.len()) + 1,
                Location::Query(label.range.clone()),
            )?;
            if previous.value == label.value {
                return Err(QueryError::query(
                    label.range.clone(),
                    "duplicate record label",
                ));
            }
        }
        let types = query_value_types(expression, &facts, &mut queries, budget)?;
        positions = crate::value::extend_type_positions(
            &positions,
            &types,
            budget,
            &Location::Query(expression.range.clone()),
        )?;
    }
    admit_slots::<ValueType>(positions.len(), budget, range)?;
    let mut records = Vec::with_capacity(positions.len());
    for position in positions {
        admit_slots::<(String, ValueType)>(position.len(), budget, range)?;
        let mut fields = Vec::with_capacity(position.len());
        for ((label, _), ty) in projection.fields.iter().zip(position) {
            budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
            fields.push((label.value.clone(), ty));
        }
        records.push(ValueType::Record(fields));
    }
    admit_slots::<ValueType>(1, budget, range)?;
    Ok(vec![ValueType::Array(ArrayType::Elements(records))])
}

/// Evaluate fields against each singleton native subject, preserving row order
/// and duplicates. All static branches are checked before obtaining subjects.
///
/// # Errors
/// Located preflight/runtime failures restore scopes without refunding the request allowance.
pub fn evaluate_projection<C, H: ProjectionHost<C>>(
    projection: &Projection<Expression<H::Stage, H::SetOperator, H::Emitter>>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<ConstructedValue, EvaluationError<H::Error>> {
    let range = &projection.subject.range;
    budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
        let input_kind = host.kind(&input, budget, range)?;
        let scope = scope_kinds(host, bindings, budget, range)?;
        projection_types(projection, host, input_kind.clone(), scope, budget)?;
        let subject_kind = host.subject_kind(&input_kind, budget, range)?;
        let subjects = host.subjects(input, context, budget, range)?;
        budget.charge_results(
            subjects.len().checked_add(1).ok_or_else(|| {
                QueryError::query(range.clone(), "result limit exceeded")
                    .with_category(QueryErrorCategory::ResultLimit)
            })?,
            Location::Query(range.clone()),
        )?;
        admit_slots::<ConstructedValue>(subjects.len(), budget, range)?;
        let mut rows = Vec::with_capacity(subjects.len());
        for subject in subjects {
            budget.charge(
                projection.subject.value.len(),
                Location::Query(range.clone()),
            )?;
            let local = bindings.with_local(
                Located::new(projection.subject.value.clone(), range.clone()),
                subject,
                budget,
            )?;
            let input = local.get(&projection.subject.value, range.clone())?;
            let scope = scope_kinds(host, &local, budget, range)?;
            let facts = value_facts(host, &scope, budget, range)?;
            let mut queries = NativeValues {
                host,
                input_kind: subject_kind.clone(),
                input: Some(input),
                context: Some(&mut *context),
                bindings: Some(&local),
                scope,
            };
            budget.charge_results(projection.fields.len() + 1, Location::Query(range.clone()))?;
            admit_slots::<(String, ConstructedValue)>(projection.fields.len(), budget, range)?;
            let mut fields = Vec::with_capacity(projection.fields.len());
            for (label, expression) in &projection.fields {
                budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
                let value = evaluate_query_value(expression, &facts, &mut queries, budget)?;
                fields.push((label.value.clone(), value));
            }
            rows.push(ConstructedValue::Record(fields));
        }
        Ok(ConstructedValue::Array(rows))
    })
}

/// Evaluate a constructed report with native operands and immutable outer scope.
///
/// # Errors
/// Static and runtime failures share the existing native evaluation allowance.
pub fn evaluate_native_value<C, H: ProjectionHost<C>>(
    expression: &ValueExpression<Expression<H::Stage, H::SetOperator, H::Emitter>>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<ConstructedValue, EvaluationError<H::Error>> {
    let scope = scope_kinds(host, bindings, budget, &expression.range)?;
    let facts = value_facts(host, &scope, budget, &expression.range)?;
    let input_kind = host.kind(&input, budget, &expression.range)?;
    let mut queries = NativeValues {
        host,
        input_kind,
        input: Some(&input),
        context: Some(context),
        bindings: Some(bindings),
        scope,
    };
    evaluate_query_value(expression, &facts, &mut queries, budget)
}

/// Infer a report's declared facts without executing a native operand.
///
/// # Errors
/// All scopes, branches, native transitions and effects are checked, including empty sources.
pub fn native_value_types<C, H: ProjectionHost<C>>(
    expression: &ValueExpression<Expression<H::Stage, H::SetOperator, H::Emitter>>,
    host: &H,
    input_kind: H::Kind,
    scope: BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<Vec<ValueType>, QueryError> {
    let facts = value_facts(host, &scope, budget, &expression.range)?;
    let mut queries = NativeValues::<C, H> {
        host,
        input_kind,
        input: None,
        context: None,
        bindings: None,
        scope,
    };
    query_value_types(expression, &facts, &mut queries, budget)
}
