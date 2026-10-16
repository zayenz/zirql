pub(crate) mod prepared;
use prepared::PreparedNative;

use std::{collections::BTreeMap, ops::Range};

use crate::{
    ArrayType, Bindings, Budget, ConstructedValue, EvaluationError, Expression, ExpressionKind,
    ExpressionStep, Located, Location, ProgramHost, QueryError, QueryErrorCategory,
    ValueExpression, ValueQueryHost, ValueType, copy_constructed_value, copy_value_types,
    query_value_types,
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

/// Scalar/native primitive arguments declared by one stage, with their relative kinds.
pub type NativeValueArguments<'s, C, H> = Vec<(
    &'s ValueExpression<NativeExpression<C, H>>,
    <H as ProjectionHost<C>>::Kind,
)>;

/// One embedded value with additional lexical names introduced by its native
/// construct. For example, a match source sees only earlier capture clauses.
/// These are preparation facts, not caller values or a replacement environment.
pub struct ScopedNativeArgument<'a, Q, K> {
    pub expression: &'a ValueExpression<Q>,
    pub relative: Option<K>,
    pub locals: Vec<(Located<String>, K)>,
}
pub type ScopedNativeValueArguments<'a, C, H> =
    Vec<ScopedNativeArgument<'a, NativeExpression<C, H>, <H as ProjectionHost<C>>::Kind>>;

/// An immutable native argument retaining its owner after a function frame ends.
/// Declaration metadata describes the whole admitted value; it does not replace
/// the host's current-document checks at a later native boundary.
pub struct NativeRead<V, K> {
    pub(crate) value: std::sync::Arc<V>,
    pub(crate) declaration: Option<crate::DeclaredType<K>>,
}
impl<V, K> NativeRead<V, K> {
    pub fn value(&self) -> &V {
        &self.value
    }
    pub fn declared_type(&self) -> Option<&crate::DeclaredType<K>> {
        self.declaration.as_ref()
    }
    pub fn into_shared(self) -> std::sync::Arc<V> {
        self.value
    }
}

/// Borrowed only while a prepared native stage executes. The runner retains the
/// exact current context and lexical scope; adapters cannot rebind its context.
pub trait NativeArguments<C, H: ProjectionHost<C>> {
    fn parts(&mut self) -> (&mut C, &Bindings<H::Value>);
    /// Evaluate a declared body only when its native argument is selected.
    ///
    /// # Errors
    /// Undeclared bodies, new-data/native refusals and resource failures retain
    /// their original locations and cumulative spending.
    fn evaluate(
        &mut self,
        expression: &ValueExpression<NativeExpression<C, H>>,
        relative_input: H::Value,
        budget: &mut Budget,
    ) -> Result<crate::ConstructedRead, EvaluationError<H::Error>>;
    /// Evaluate a selected declared argument while retaining native ownership.
    /// Existing scalar-only runners reject this opt-in bridge.
    ///
    /// # Errors
    /// Undeclared arguments, unavailable native execution and limits are located.
    #[expect(
        clippy::type_complexity,
        reason = "The native argument retains the adapter's value, kind and error types"
    )]
    fn evaluate_native(
        &mut self,
        expression: &ValueExpression<NativeExpression<C, H>>,
        _relative_input: H::Value,
        _budget: &mut Budget,
    ) -> Result<NativeRead<H::Value, H::Kind>, EvaluationError<H::Error>> {
        Err(QueryError::query(
            expression.range.clone(),
            "native arguments require a prepared function program",
        )
        .into())
    }

    /// Evaluate a declared match source or guard against the common match
    /// executor's immutable clause bindings, retaining this active runner.
    /// The corresponding scoped argument must declare every extra name.
    ///
    /// # Errors
    /// Unsupported runners, undeclared values, scope and resource failures retain locations.
    #[expect(
        clippy::type_complexity,
        reason = "The argument retains the adapter's native value, kind and error types"
    )]
    fn evaluate_match_value(
        &mut self,
        expression: &ValueExpression<NativeExpression<C, H>>,
        _relative_input: H::Value,
        _bindings: &Bindings<H::Value>,
        _budget: &mut Budget,
    ) -> Result<NativeRead<H::Value, H::Kind>, EvaluationError<H::Error>> {
        Err(QueryError::query(
            expression.range.clone(),
            "match arguments require a prepared function program",
        )
        .into())
    }

    /// Project this stage's declared fields through the active prepared runner.
    /// Declare every field with [`ProjectionHost::value_arguments`] and give it
    /// the singleton subject's relative kind and local name through
    /// [`ProjectionHost::scoped_value_arguments`]. The runner admits all fields
    /// before selecting subjects, then shares its current lexical bindings and
    /// cumulative budget with the common row executor. Hosts retain native
    /// ownership checks and explicit scalar conversion.
    ///
    /// # Errors
    /// Unsupported runners, undeclared fields, shadowing, invalid scalar types
    /// and resource failures retain their locations and active function calls.
    fn evaluate_projection(
        &mut self,
        projection: &Projection<NativeExpression<C, H>>,
        _input: H::Value,
        _budget: &mut Budget,
    ) -> Result<ConstructedResult, EvaluationError<H::Error>> {
        Err(QueryError::query(
            projection.subject.range.clone(),
            "projection arguments require a prepared function program",
        )
        .into())
    }
}

/// Domain facts and explicit scalar conversions for shared native projection.
/// Hooks describe native stages; the core owns query composition and pure scope.
pub trait ProjectionHost<C>: ProgramHost<C> {
    type Kind: Clone + PartialEq;

    /// Expose embedded primitive argument bodies without evaluating their data.
    fn value_arguments<'s>(
        &self,
        _stage: &'s Self::Stage,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<NativeValueArguments<'s, C, Self>, QueryError> {
        Ok(Vec::new())
    }
    /// Describe exact clause-local scope for declared native arguments. Ordinary
    /// stages retain their current arguments and relative-input policy.
    fn scoped_value_arguments<'s>(
        &self,
        stage: &'s Self::Stage,
        _input: &Self::Kind,
        _scope: &BTreeMap<String, Self::Kind>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<ScopedNativeValueArguments<'s, C, Self>, QueryError> {
        let arguments = self.value_arguments(stage, budget, range)?;
        admit_slots::<ScopedNativeArgument<'_, NativeExpression<C, Self>, Self::Kind>>(
            arguments.len(),
            budget,
            range,
        )?;
        Ok(arguments
            .into_iter()
            .map(|(expression, kind)| ScopedNativeArgument {
                expression,
                relative: (!self.is_explicit_source(stage)).then_some(kind),
                locals: Vec::new(),
            })
            .collect())
    }
    /// Execute a stage with its invocation-local prepared argument runner.
    /// Existing hosts keep their ordinary stage implementation.
    fn evaluate_stage_with_arguments(
        &self,
        stage: &Self::Stage,
        input: Self::Value,
        arguments: &mut dyn NativeArguments<C, Self>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>
    where
        Self: Sized,
    {
        let (context, bindings) = arguments.parts();
        self.evaluate_stage(stage, input, context, bindings, budget, range)
    }
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
    /// Admit native set operand kinds before execution, including empty operands
    /// and unused branches. The default requires equal kinds; hosts can also
    /// refuse kinds without a native membership identity.
    fn set_kind(
        &self,
        left: &Self::Kind,
        right: &Self::Kind,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Kind, QueryError> {
        if left != right {
            return Err(QueryError::query(
                range.clone(),
                "set operands require the same native kind",
            ));
        }
        Ok(left.clone())
    }
    /// A pure stage that supplies its own explicit native input through declared
    /// arguments, rather than reading the surrounding pipeline input. The runner
    /// supplies an absent placeholder which this stage must ignore.
    fn is_explicit_source(&self, _stage: &Self::Stage) -> bool {
        false
    }
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
    /// Declared positions of an explicitly exposed native tuple view.
    fn tuple_view_kind<'k>(&self, _kind: &'k Self::Kind) -> Option<&'k [Self::Kind]> {
        None
    }
    fn project_value(
        &self,
        value: Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<ConstructedValue, EvaluationError<Self::Error>>;
    /// Share a selected immutable constructed binding when the host retains it.
    /// The hook must preserve current context/handle checks. Runtime admission
    /// still inspects the selected value; the default uses owned projection.
    fn share_constructed(
        &self,
        _value: &Self::Value,
        _context: &mut C,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Option<std::sync::Arc<ConstructedValue>>, EvaluationError<Self::Error>> {
        Ok(None)
    }

    /// Lift a shared immutable scalar argument into the native host. The default
    /// materializes the owned value required by the existing lifting hook.
    fn lift_shared_constructed(
        &self,
        value: std::sync::Arc<ConstructedValue>,
        types: Vec<ValueType>,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>> {
        let value = copy_constructed_value(&value, budget, Location::Query(range.clone()))?;
        self.lift_constructed(value, types, context, budget, range)
    }
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
                let mut left = native_expression_kind(operand, host, input, scope, budget)?;
                while let Some((right, range)) = pending.pop() {
                    let right = native_expression_kind(right, host, input, scope, budget)?;
                    left = host.set_kind(&left, &right, budget, range)?;
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

pub(crate) fn admit_slots<T>(
    len: usize,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<(), QueryError> {
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

enum NativePreparation<'a> {
    Checking(&'a mut BTreeMap<usize, PreparedNative>),
    Running(&'a BTreeMap<usize, PreparedNative>),
}
impl NativePreparation<'_> {
    fn checking(&mut self) -> &mut BTreeMap<usize, PreparedNative> {
        match self {
            Self::Checking(values) => values,
            Self::Running(_) => unreachable!("runtime argument bodies are already prepared"),
        }
    }
    fn values(&self) -> &BTreeMap<usize, PreparedNative> {
        match self {
            Self::Checking(values) => values,
            Self::Running(values) => values,
        }
    }
}

type DeclaredArguments<'q, C, H> = Vec<(
    &'q ValueExpression<NativeExpression<C, H>>,
    Option<<H as ProjectionHost<C>>::Kind>,
)>;

pub(crate) fn declared_arguments<'q, C, H: ProjectionHost<C>>(
    query: &'q NativeExpression<C, H>,
    host: &H,
    budget: &mut Budget,
) -> Result<DeclaredArguments<'q, C, H>, QueryError> {
    enum Pending<'q, S, O, E> {
        Expression(&'q Expression<S, O, E>),
        Step(&'q Located<ExpressionStep<S, E>>),
    }
    admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter>>(1, budget, &query.range)?;
    let mut pending = vec![Pending::Expression(query)];
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        match node {
            Pending::Step(step) => {
                if let ExpressionStep::Stage(stage) = &step.value {
                    let arguments = host.value_arguments(stage, budget, &step.range)?;
                    admit_slots::<(&ValueExpression<NativeExpression<C, H>>, H::Kind)>(
                        arguments.len(),
                        budget,
                        &step.range,
                    )?;
                    let relative = !host.is_explicit_source(stage);
                    result.extend(
                        arguments
                            .into_iter()
                            .map(|(value, kind)| (value, relative.then_some(kind))),
                    );
                }
            }
            Pending::Expression(expression) => match &expression.value {
                ExpressionKind::Input | ExpressionKind::Binding(_) => {}
                ExpressionKind::Group(value) | ExpressionKind::Field { value, .. } => {
                    admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter>>(
                        1,
                        budget,
                        &expression.range,
                    )?;
                    pending.push(Pending::Expression(value));
                }
                ExpressionKind::Sequence { input, steps } => {
                    admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter>>(
                        steps.len().checked_add(1).ok_or_else(|| {
                            QueryError::query(
                                expression.range.clone(),
                                "argument metadata size overflow",
                            )
                        })?,
                        budget,
                        &expression.range,
                    )?;
                    pending.extend(steps.iter().rev().map(Pending::Step));
                    pending.push(Pending::Expression(input));
                }
                ExpressionKind::Set { left, right, .. } => {
                    admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter>>(
                        2,
                        budget,
                        &expression.range,
                    )?;
                    pending.push(Pending::Expression(right));
                    pending.push(Pending::Expression(left));
                }
                ExpressionKind::Record(fields) => {
                    admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter>>(
                        fields.len(),
                        budget,
                        &expression.range,
                    )?;
                    pending.extend(
                        fields
                            .iter()
                            .rev()
                            .map(|(_, value)| Pending::Expression(value)),
                    );
                }
            },
        }
    }
    Ok(result)
}

pub(crate) fn scoped_arguments<'q, C, H: ProjectionHost<C>>(
    query: &'q NativeExpression<C, H>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<ScopedNativeValueArguments<'q, C, H>, QueryError> {
    enum Pending<'q, S, O, E, K> {
        Expression(&'q Expression<S, O, E>, K),
        Stage(&'q Located<ExpressionStep<S, E>>, K),
    }
    admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter, H::Kind>>(
        1,
        budget,
        &query.range,
    )?;
    let mut pending = vec![Pending::Expression(query, input.clone())];
    let mut result = Vec::new();
    while let Some(next) = pending.pop() {
        match next {
            Pending::Stage(step, kind) => {
                if let ExpressionStep::Stage(stage) = &step.value {
                    let arguments =
                        host.scoped_value_arguments(stage, &kind, scope, budget, &step.range)?;
                    admit_slots::<ScopedNativeArgument<'_, NativeExpression<C, H>, H::Kind>>(
                        arguments.len(),
                        budget,
                        &step.range,
                    )?;
                    result.extend(arguments);
                }
            }
            Pending::Expression(expression, input) => {
                budget.charge(1, Location::Query(expression.range.clone()))?;
                let count = match &expression.value {
                    ExpressionKind::Sequence { steps, .. } => steps.len() + 1,
                    ExpressionKind::Set { .. } => 2,
                    ExpressionKind::Record(fields) => fields.len(),
                    ExpressionKind::Group(_) | ExpressionKind::Field { .. } => 1,
                    _ => 0,
                };
                admit_slots::<Pending<'_, H::Stage, H::SetOperator, H::Emitter, H::Kind>>(
                    count,
                    budget,
                    &expression.range,
                )?;
                match &expression.value {
                    ExpressionKind::Sequence {
                        input: source,
                        steps,
                    } => {
                        let mut stages = Vec::new();
                        admit_slots::<(&Located<ExpressionStep<H::Stage, H::Emitter>>, H::Kind)>(
                            steps.len(),
                            budget,
                            &expression.range,
                        )?;
                        let mut kind = native_expression_kind(source, host, &input, scope, budget)?;
                        for step in steps {
                            if let ExpressionStep::Stage(stage) = &step.value {
                                stages.push((step, kind.clone()));
                                kind = host.stage_kind(stage, &kind, scope, budget, &step.range)?;
                            }
                        }
                        pending.extend(
                            stages
                                .into_iter()
                                .rev()
                                .map(|(step, kind)| Pending::Stage(step, kind)),
                        );
                        pending.push(Pending::Expression(source, input));
                    }
                    ExpressionKind::Group(value) | ExpressionKind::Field { value, .. } => {
                        pending.push(Pending::Expression(value, input))
                    }
                    ExpressionKind::Set { left, right, .. } => {
                        pending.push(Pending::Expression(right, input.clone()));
                        pending.push(Pending::Expression(left, input));
                    }
                    ExpressionKind::Record(fields) => pending.extend(
                        fields
                            .iter()
                            .rev()
                            .map(|(_, value)| Pending::Expression(value, input.clone())),
                    ),
                    ExpressionKind::Input | ExpressionKind::Binding(_) => {}
                }
            }
        }
    }
    Ok(result)
}

struct NativeValues<'a, C, H: ProjectionHost<C>> {
    host: &'a H,
    input_kind: H::Kind,
    input: Option<&'a H::Value>,
    context: Option<&'a mut C>,
    bindings: Option<&'a Bindings<H::Value>>,
    scope: &'a BTreeMap<String, H::Kind>,
    outer_names: &'a [&'a str],
    outer: &'a [&'a H::Value],
    subject_name: Option<&'a str>,
    prepared: NativePreparation<'a>,
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
        let mut scope = copy_scope::<C, H>(self.scope, budget, &query.range)?;
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
        let prepared = PreparedNative::resolve::<C, H>(
            query,
            self.host,
            self.outer_names,
            self.subject_name,
            locals,
            budget,
        )?;
        budget.charge(1, Location::Query(query.range.clone()))?;
        self.prepared
            .checking()
            .insert(query as *const _ as usize, prepared);
        let arguments =
            scoped_arguments::<C, H>(query, self.host, &self.input_kind, &scope, budget)?;
        for argument in arguments {
            let mut scope = copy_scope::<C, H>(&scope, budget, &query.range)?;
            for (name, kind) in argument.locals {
                budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
                if scope.insert(name.value, kind).is_some() {
                    return Err(QueryError::query(name.range, "binding is already defined"));
                }
            }
            let facts = value_facts(self.host, &scope, budget, &query.range)?;
            admit_slots::<&str>(scope.len(), budget, &query.range)?;
            let outer_names = scope.keys().map(String::as_str).collect::<Vec<_>>();
            let mut queries = NativeValues {
                host: self.host,
                input_kind: argument
                    .relative
                    .unwrap_or_else(|| self.host.constructed_kind(vec![ValueType::Unestablished])),
                input: None,
                context: None,
                bindings: None,
                scope: &scope,
                outer_names: &outer_names,
                outer: &[],
                subject_name: None,
                prepared: NativePreparation::Checking(self.prepared.checking()),
            };
            query_value_types(argument.expression, &facts, &mut queries, budget)?;
        }
        self.host
            .scalar_types(&kind, budget, &query.range)?
            .ok_or_else(|| {
                QueryError::query(
                    query.range.clone(),
                    "native handles require an explicit scalar projection",
                )
            })
    }
    fn query_arguments<'q>(
        &self,
        query: &'q NativeExpression<C, H>,
        budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Vec<&'q ValueExpression<NativeExpression<C, H>>>, QueryError> {
        let arguments = declared_arguments::<C, H>(query, self.host, budget)?;
        admit_slots::<&ValueExpression<NativeExpression<C, H>>>(
            arguments.len(),
            budget,
            &query.range,
        )?;
        Ok(arguments
            .into_iter()
            .map(|(expression, _)| expression)
            .collect())
    }
    fn evaluate_query(
        &mut self,
        query: &Expression<H::Stage, H::SetOperator, H::Emitter>,
        locals: &[(&str, ConstructedValue)],
        budget: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error> {
        admit_slots::<std::sync::Arc<ConstructedValue>>(locals.len(), budget, &query.range)?;
        let mut shared = Vec::with_capacity(locals.len());
        for (name, value) in locals {
            shared.push((
                *name,
                std::sync::Arc::new(copy_constructed_value(
                    value,
                    budget,
                    Location::Query(query.range.clone()),
                )?),
            ));
        }
        self.evaluate_query_with_shared_locals(query, &shared, budget, &query.range)
    }
    fn evaluate_query_with_shared_locals(
        &mut self,
        query: &NativeExpression<C, H>,
        locals: &[(&str, std::sync::Arc<ConstructedValue>)],
        budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<ConstructedValue, Self::Error> {
        self.evaluate_query_with_arguments(query, locals, &mut NoArguments, budget, &query.range)
    }
    fn evaluate_query_with_arguments(
        &mut self,
        query: &NativeExpression<C, H>,
        locals: &[(&str, std::sync::Arc<ConstructedValue>)],
        arguments: &mut dyn crate::ValueArguments<NativeExpression<C, H>, Self::Error>,
        budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<ConstructedValue, Self::Error> {
        let context = self.context.as_deref_mut().expect("runtime native context");
        let mut scoped = None;
        for (name, value) in locals {
            admit_slots::<ValueType>(1, budget, &query.range)?;
            let types = vec![crate::constructed_value_type(
                value,
                budget,
                Location::Query(query.range.clone()),
            )?];
            let value = self.host.lift_shared_constructed(
                std::sync::Arc::clone(value),
                types,
                context,
                budget,
                &query.range,
            )?;
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
        let bindings = scoped
            .as_ref()
            .unwrap_or(self.bindings.expect("runtime native bindings"));
        admit_slots::<H::Value>(locals.len(), budget, &query.range)?;
        let mut local_values = Vec::with_capacity(locals.len());
        for (name, _) in locals {
            local_values.push(bindings.get(name, query.range.clone())?);
        }
        let prepared = self
            .prepared
            .values()
            .get(&(query as *const _ as usize))
            .expect("native operand prepared in this invocation");
        let value = prepared.run(
            query,
            self.host,
            input,
            context,
            bindings,
            self.outer,
            self.input.expect("runtime relative input"),
            &local_values,
            arguments,
            self.prepared.values(),
            self.scope,
            budget,
        )?;
        self.host
            .project_value(value, context, budget, &query.range)
    }
    fn read_shared_binding(
        &mut self,
        name: &str,
        range: &Range<usize>,
        budget: &mut Budget,
    ) -> Result<Option<std::sync::Arc<ConstructedValue>>, Self::Error> {
        self.host.share_constructed(
            self.bindings
                .expect("runtime native bindings")
                .get(name, range.clone())?,
            self.context.as_deref_mut().expect("runtime native context"),
            budget,
            range,
        )
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

struct NoArguments;
impl<Q, E: From<QueryError>> crate::ValueArguments<Q, E> for NoArguments {
    fn evaluate(
        &mut self,
        expression: &ValueExpression<Q>,
        _: &mut dyn ValueQueryHost<Q, Error = E>,
        _: &mut Budget,
    ) -> Result<crate::ConstructedRead, E> {
        Err(QueryError::query(
            expression.range.clone(),
            "argument was not prepared in this invocation",
        )
        .into())
    }
}

struct StageArguments<'a, 'v, C, H: ProjectionHost<C>> {
    host: &'a H,
    context: &'a mut C,
    bindings: &'a Bindings<H::Value>,
    scope: &'a BTreeMap<String, H::Kind>,
    prepared: &'a BTreeMap<usize, PreparedNative>,
    declared: &'a [usize],
    values: &'v mut dyn crate::ValueArguments<NativeExpression<C, H>, EvaluationError<H::Error>>,
}
impl<C, H: ProjectionHost<C>> NativeArguments<C, H> for StageArguments<'_, '_, C, H> {
    fn parts(&mut self) -> (&mut C, &Bindings<H::Value>) {
        (self.context, self.bindings)
    }
    fn evaluate_projection(
        &mut self,
        projection: &Projection<NativeExpression<C, H>>,
        input: H::Value,
        budget: &mut Budget,
    ) -> Result<ConstructedResult, EvaluationError<H::Error>> {
        evaluate_projection_with_types(
            projection,
            self.host,
            input,
            self.context,
            self.bindings,
            budget,
        )
    }
    fn evaluate(
        &mut self,
        expression: &ValueExpression<NativeExpression<C, H>>,
        input: H::Value,
        budget: &mut Budget,
    ) -> Result<crate::ConstructedRead, EvaluationError<H::Error>> {
        let id = expression as *const _ as usize;
        budget.charge(
            self.declared.len() + 1,
            Location::Query(expression.range.clone()),
        )?;
        if !self.declared.contains(&id) {
            return Err(QueryError::query(
                expression.range.clone(),
                "argument was not declared by this native stage",
            )
            .into());
        }
        admit_slots::<(&str, &H::Value)>(self.bindings.iter().count(), budget, &expression.range)?;
        let outer_names = self
            .bindings
            .iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        let outer = self
            .bindings
            .iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>();
        let input_kind = self.host.kind(&input, budget, &expression.range)?;
        let mut queries = NativeValues {
            host: self.host,
            input_kind,
            input: Some(&input),
            context: Some(&mut *self.context),
            bindings: Some(self.bindings),
            scope: self.scope,
            outer_names: &outer_names,
            outer: &outer,
            subject_name: None,
            prepared: NativePreparation::Running(self.prepared),
        };
        self.values.evaluate(expression, &mut queries, budget)
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
    let mut prepared = BTreeMap::new();
    admit_slots::<&str>(scope.len().saturating_sub(1), budget, range)?;
    let outer_names = scope
        .keys()
        .filter(|name| *name != &projection.subject.value)
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut queries = NativeValues::<C, H> {
        prepared: NativePreparation::Checking(&mut prepared),
        outer_names: &outer_names,
        outer: &[],
        subject_name: Some(&projection.subject.value),
        host,
        input_kind: subject_kind,
        input: None,
        context: None,
        bindings: None,
        scope: &scope,
    };
    admit_slots::<Vec<ValueType>>(1, budget, range)?;
    let mut positions = vec![Vec::new()];
    for (index, (_, expression)) in projection.fields.iter().enumerate() {
        check_projection_label(projection, index, budget)?;
        let types = query_value_types(expression, &facts, &mut queries, budget)?;
        positions = crate::value::extend_type_positions(
            &positions,
            &types,
            budget,
            &Location::Query(expression.range.clone()),
        )?;
    }
    projection_result_types(projection, positions, budget)
}

pub(crate) fn check_projection_label<Q>(
    projection: &Projection<Q>,
    index: usize,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let label = &projection.fields[index].0;
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
    Ok(())
}

pub(crate) fn projection_result_types<Q>(
    projection: &Projection<Q>,
    positions: Vec<Vec<ValueType>>,
    budget: &mut Budget,
) -> Result<Vec<ValueType>, QueryError> {
    let range = &projection.subject.range;
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
    Ok(evaluate_projection_with_types(projection, host, input, context, bindings, budget)?.value)
}

/// Owned report and result facts prepared in the same invocation.
#[derive(Debug, PartialEq)]
pub struct ConstructedResult {
    pub value: ConstructedValue,
    pub types: Vec<ValueType>,
}

/// Prepare every written field once, then execute its resolved slots for each
/// current native subject. Preparation cannot escape this context or invocation.
///
/// # Errors
/// All-written preflight runs before enumeration; runtime/native failures retain
/// their locations and cumulative spending while current frames are released.
pub fn evaluate_projection_with_types<C, H: ProjectionHost<C>>(
    projection: &Projection<Expression<H::Stage, H::SetOperator, H::Emitter>>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<ConstructedResult, EvaluationError<H::Error>> {
    let range = &projection.subject.range;
    budget.within_evaluation(1, Location::Query(range.clone()), |budget| {
        let input_kind = host.kind(&input, budget, range)?;
        let mut scope = scope_kinds(host, bindings, budget, range)?;
        if scope.contains_key(&projection.subject.value) {
            return Err(QueryError::query(
                range.clone(),
                "projection subject shadows an existing binding",
            )
            .into());
        }
        budget.charge(
            projection.subject.value.len() + 1,
            Location::Query(range.clone()),
        )?;
        let subject_kind = host.subject_kind(&input_kind, budget, range)?;
        scope.insert(projection.subject.value.clone(), subject_kind.clone());
        let facts = value_facts(host, &scope, budget, range)?;
        let mut prepared_native = BTreeMap::new();
        admit_slots::<(&str, &H::Value)>(bindings.iter().count(), budget, range)?;
        let outer_names = bindings.iter().map(|(name, _)| name).collect::<Vec<_>>();
        let outer = bindings.iter().map(|(_, value)| value).collect::<Vec<_>>();
        let mut queries = NativeValues {
            host,
            input_kind: subject_kind.clone(),
            input: None,
            context: None,
            bindings: None,
            scope: &scope,
            outer_names: &outer_names,
            outer: &outer,
            subject_name: Some(&projection.subject.value),
            prepared: NativePreparation::Checking(&mut prepared_native),
        };
        admit_slots::<crate::value::PreparedQueryValue<'_, NativeExpression<C, H>>>(
            projection.fields.len(),
            budget,
            range,
        )?;
        let mut fields = Vec::with_capacity(projection.fields.len());
        let mut positions = vec![Vec::new()];
        for (index, (_, expression)) in projection.fields.iter().enumerate() {
            check_projection_label(projection, index, budget)?;
            let field =
                crate::value::prepare_query_value(expression, &facts, &mut queries, budget)?;
            positions = crate::value::extend_type_positions(
                &positions,
                &field.types,
                budget,
                &Location::Query(expression.range.clone()),
            )?;
            fields.push(field);
        }
        let types = projection_result_types(projection, positions, budget)?;
        admit_slots::<(crate::value::prepared::Frame<'_>, Option<usize>)>(
            fields.len(),
            budget,
            range,
        )?;
        let mut frames = Vec::with_capacity(fields.len());
        for field in &fields {
            let frame = field.frame(budget)?;
            let subject = frame.named_slot(&projection.subject.value, budget, range)?;
            frames.push((frame, subject));
        }
        let subjects = host.subjects(input, context, budget, range)?;
        let value = projection_rows(
            projection,
            subjects,
            bindings,
            context,
            budget,
            |index, local, context, budget| {
                let mut queries = NativeValues {
                    host,
                    input_kind: subject_kind.clone(),
                    input: Some(local.get(&projection.subject.value, range.clone())?),
                    bindings: Some(local),
                    context: Some(context),
                    scope: &scope,
                    outer_names: &outer_names,
                    outer: &outer,
                    subject_name: Some(&projection.subject.value),
                    prepared: NativePreparation::Running(&prepared_native),
                };
                let (frame, subject_slot) = &mut frames[index];
                frame.reset(*subject_slot);
                let result = fields[index].evaluate(&mut queries, frame, budget);
                frame.reset(*subject_slot);
                result
            },
        )?;
        Ok(ConstructedResult { value, types })
    })
}

pub(crate) fn projection_rows<Q, V, C, E: From<QueryError>>(
    projection: &Projection<Q>,
    subjects: Vec<V>,
    bindings: &Bindings<V>,
    context: &mut C,
    budget: &mut Budget,
    mut field: impl FnMut(usize, &Bindings<V>, &mut C, &mut Budget) -> Result<ConstructedValue, E>,
) -> Result<ConstructedValue, E> {
    let range = &projection.subject.range;
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
        budget.charge_results(projection.fields.len() + 1, Location::Query(range.clone()))?;
        admit_slots::<(String, ConstructedValue)>(projection.fields.len(), budget, range)?;
        let mut values = Vec::with_capacity(projection.fields.len());
        for (index, (label, _)) in projection.fields.iter().enumerate() {
            budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
            values.push((label.value.clone(), field(index, &local, context, budget)?));
        }
        rows.push(ConstructedValue::Record(values));
    }
    Ok(ConstructedValue::Array(rows))
}

/// Evaluate a constructed report with native operands and immutable outer scope.
///
/// # Errors
/// Static and runtime failures share the existing native evaluation allowance.
pub fn evaluate_native_value<C, H: ProjectionHost<C>>(
    expression: &ValueExpression<NativeExpression<C, H>>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<ConstructedValue, EvaluationError<H::Error>> {
    Ok(evaluate_native_value_with_types(expression, host, input, context, bindings, budget)?.value)
}

/// Prepare a native value and return its owned result with its prepared facts.
///
/// # Errors
/// Structural/name/type preflight and selected runtime admission use the active
/// allowance and retain the original scalar/native source locations.
pub fn evaluate_native_value_with_types<C, H: ProjectionHost<C>>(
    expression: &ValueExpression<NativeExpression<C, H>>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<ConstructedResult, EvaluationError<H::Error>> {
    let scope = scope_kinds(host, bindings, budget, &expression.range)?;
    let facts = value_facts(host, &scope, budget, &expression.range)?;
    let input_kind = host.kind(&input, budget, &expression.range)?;
    let mut prepared_native = BTreeMap::new();
    admit_slots::<(&str, &H::Value)>(bindings.iter().count(), budget, &expression.range)?;
    let outer_names = bindings.iter().map(|(name, _)| name).collect::<Vec<_>>();
    let outer = bindings.iter().map(|(_, value)| value).collect::<Vec<_>>();
    let mut queries = NativeValues {
        host,
        input_kind: input_kind.clone(),
        input: None,
        context: None,
        bindings: None,
        scope: &scope,
        outer_names: &outer_names,
        outer: &outer,
        subject_name: None,
        prepared: NativePreparation::Checking(&mut prepared_native),
    };
    let prepared = crate::value::prepare_query_value(expression, &facts, &mut queries, budget)?;
    let mut frame = prepared.frame(budget)?;
    let mut queries = NativeValues {
        host,
        input_kind,
        input: Some(&input),
        context: Some(context),
        bindings: Some(bindings),
        scope: &scope,
        outer_names: &outer_names,
        outer: &outer,
        subject_name: None,
        prepared: NativePreparation::Running(&prepared_native),
    };
    let value = prepared.evaluate(&mut queries, &mut frame, budget)?;
    Ok(ConstructedResult {
        value,
        types: prepared.types,
    })
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
    let mut prepared = BTreeMap::new();
    admit_slots::<&str>(scope.len(), budget, &expression.range)?;
    let outer_names = scope.keys().map(String::as_str).collect::<Vec<_>>();
    let mut queries = NativeValues::<C, H> {
        prepared: NativePreparation::Checking(&mut prepared),
        outer_names: &outer_names,
        outer: &[],
        subject_name: None,
        host,
        input_kind,
        input: None,
        context: None,
        bindings: None,
        scope: &scope,
    };
    query_value_types(expression, &facts, &mut queries, budget)
}
