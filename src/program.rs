use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::{
    Budget, ConstructedValue, EvaluationError, Located, Location, QueryError, QueryErrorCategory,
    ValueType, copy_constructed_value,
};

/// Required caller inputs support exactly these three scalar types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterType {
    Int,
    String,
    Bool,
}

impl From<ParameterType> for ValueType {
    fn from(ty: ParameterType) -> Self {
        match ty {
            ParameterType::Int => Self::Integer,
            ParameterType::String => Self::String,
            ParameterType::Bool => Self::Boolean,
        }
    }
}

/// Immutable request metadata retained beside a host's lowered program.
/// Text frontends introduce the name into scope at its written declaration,
/// checking host reserved names and later binding/generator/projection collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterDeclaration {
    pub name: Located<String>,
    pub ty: Located<ParameterType>,
}

/// Borrowed caller values. CLI hosts collect raw arguments without decoding or
/// substituting them into source; library callers supply exact scalar values.
#[derive(Clone, Copy, Debug)]
pub enum ParameterInputs<'a> {
    Typed(&'a [(String, ConstructedValue)]),
    Cli(&'a [String]),
}

/// Admit a complete required argument set before any domain execution.
/// Returned values follow declaration order and use declaration name ranges;
/// hosts lift them into their native wrapper and install them with [`Bindings::new`].
/// Admission, preflight, execution and rendering must share `budget`.
///
/// CLI inputs split at the first `=`. Strings are verbatim, including empty
/// strings. Integers are optional `+`/`-` followed by ASCII decimal digits fitting
/// `i64`; Booleans are exactly `true` or `false`. No trimming or unescaping occurs.
/// Empty declarations and empty inputs spend nothing, preserving literal queries.
///
/// # Errors
/// Empty, duplicate or ambient declaration names, duplicate/missing/extra inputs,
/// wrong scalar types and exhausted allowances fail without returning a partial
/// set. Missing values point to the name and wrong types to the declaration type;
/// other input errors use `request_range`, not fabricated argument coordinates.
pub fn admit_parameters(
    declarations: &[ParameterDeclaration],
    inputs: ParameterInputs<'_>,
    ambient_names: &[&str],
    request_range: Range<usize>,
    budget: &mut Budget,
) -> Result<Vec<(Located<String>, ConstructedValue)>, QueryError> {
    if declarations.is_empty()
        && match inputs {
            ParameterInputs::Typed(values) => values.is_empty(),
            ParameterInputs::Cli(values) => values.is_empty(),
        }
    {
        return Ok(Vec::new());
    }
    // Lookup slots borrow declaration names; scalar copies are admitted below.
    parameter_slots::<(&str, ParameterSlot<'_>)>(declarations.len(), budget, &request_range)?;
    let mut values = BTreeMap::new();
    for declaration in declarations {
        let name = &declaration.name;
        budget.charge(name.value.len(), Location::Query(name.range.clone()))?;
        if name.value.is_empty() {
            return Err(QueryError::query(
                name.range.clone(),
                "parameter name is empty",
            ));
        }
        for ambient in ambient_names {
            budget.charge(
                1 + name.value.len().min(ambient.len()),
                Location::Query(name.range.clone()),
            )?;
            if name.value == *ambient {
                return Err(parameter_error(name, "name is reserved", budget)?);
            }
        }
        if values
            .insert(
                name.value.as_str(),
                ParameterSlot {
                    declaration,
                    value: None,
                },
            )
            .is_some()
        {
            return Err(parameter_error(name, "already declared", budget)?);
        }
    }
    match inputs {
        ParameterInputs::Typed(inputs) => {
            for (name, value) in inputs {
                budget.charge(name.len(), Location::Query(request_range.clone()))?;
                let slot = parameter_slot(&mut values, name, &request_range, budget)?;
                let declaration = slot.declaration;
                if !matches!(
                    (declaration.ty.value, value),
                    (ParameterType::Int, ConstructedValue::Integer(_))
                        | (ParameterType::String, ConstructedValue::String(_))
                        | (ParameterType::Bool, ConstructedValue::Boolean(_))
                ) {
                    return Err(parameter_type_error(declaration, budget)?);
                }
                slot.value = Some(copy_constructed_value(
                    value,
                    budget,
                    Location::Query(declaration.ty.range.clone()),
                )?);
            }
        }
        ParameterInputs::Cli(inputs) => {
            for argument in inputs {
                budget.charge(argument.len(), Location::Query(request_range.clone()))?;
                let Some((name, text)) = argument.split_once('=') else {
                    return Err(parameter_error(
                        &Located::new(argument.as_str(), request_range.clone()),
                        "expected name=value",
                        budget,
                    )?);
                };
                let slot = parameter_slot(&mut values, name, &request_range, budget)?;
                let declaration = slot.declaration;
                budget.charge_results(1, Location::Query(declaration.ty.range.clone()))?;
                let value = match declaration.ty.value {
                    ParameterType::Int => {
                        let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
                        let integer = (!digits.is_empty()
                            && digits.bytes().all(|byte| byte.is_ascii_digit()))
                        .then(|| text.parse::<i64>().ok())
                        .flatten();
                        let Some(integer) = integer else {
                            return Err(parameter_type_error(declaration, budget)?);
                        };
                        ConstructedValue::Integer(integer)
                    }
                    ParameterType::String => {
                        budget.charge(text.len(), Location::Query(declaration.ty.range.clone()))?;
                        ConstructedValue::String(text.to_owned())
                    }
                    ParameterType::Bool => match text {
                        "true" => ConstructedValue::Boolean(true),
                        "false" => ConstructedValue::Boolean(false),
                        _ => return Err(parameter_type_error(declaration, budget)?),
                    },
                };
                slot.value = Some(value);
            }
        }
    }
    for declaration in declarations {
        if values[declaration.name.value.as_str()].value.is_none() {
            return Err(parameter_error(
                &declaration.name,
                "missing required value",
                budget,
            )?);
        }
    }
    parameter_slots::<(Located<String>, ConstructedValue)>(
        declarations.len(),
        budget,
        &request_range,
    )?;
    let mut admitted = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        budget.charge(
            declaration.name.value.len(),
            Location::Query(declaration.name.range.clone()),
        )?;
        let value = values
            .remove(declaration.name.value.as_str())
            .and_then(|slot| slot.value)
            .expect("all parameter values were checked above");
        admitted.push((declaration.name.clone(), value));
    }
    Ok(admitted)
}

fn parameter_slots<T>(
    count: usize,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<(), QueryError> {
    if count
        .checked_mul(std::mem::size_of::<T>())
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "parameter allocation size overflow",
        ));
    }
    let units = count
        .checked_add(1)
        .ok_or_else(|| QueryError::query(range.clone(), "parameter allocation size overflow"))?;
    budget.charge(units, Location::Query(range.clone()))?;
    budget.charge_results(units, Location::Query(range.clone()))
}

struct ParameterSlot<'a> {
    declaration: &'a ParameterDeclaration,
    value: Option<ConstructedValue>,
}

fn parameter_slot<'a, 'v>(
    values: &'v mut BTreeMap<&str, ParameterSlot<'a>>,
    name: &str,
    range: &Range<usize>,
    budget: &mut Budget,
) -> Result<&'v mut ParameterSlot<'a>, QueryError> {
    let located = Located::new(name, range.clone());
    let Some(slot) = values.get_mut(name) else {
        return Err(parameter_error(&located, "is not declared", budget)?);
    };
    if slot.value.is_some() {
        return Err(parameter_error(
            &located,
            "supplied more than once",
            budget,
        )?);
    }
    Ok(slot)
}

fn parameter_type_error(
    declaration: &ParameterDeclaration,
    budget: &mut Budget,
) -> Result<QueryError, QueryError> {
    let message = match declaration.ty.value {
        ParameterType::Int => "expected decimal int fitting i64",
        ParameterType::String => "expected string",
        ParameterType::Bool => "expected bool (true or false)",
    };
    parameter_error(
        &Located::new(
            declaration.name.value.as_str(),
            declaration.ty.range.clone(),
        ),
        message,
        budget,
    )
}

fn parameter_error(
    name: &Located<impl AsRef<str>>,
    message: &str,
    budget: &mut Budget,
) -> Result<QueryError, QueryError> {
    let bytes = name
        .value
        .as_ref()
        .len()
        .checked_add(message.len())
        .and_then(|bytes| bytes.checked_add("parameter : ".len()))
        .ok_or_else(|| {
            QueryError::query(name.range.clone(), "parameter diagnostic size overflow")
        })?;
    budget.charge(bytes, Location::Query(name.range.clone()))?;
    Ok(QueryError::query(
        name.range.clone(),
        format!("parameter {}: {message}", name.value.as_ref()),
    ))
}

/// Host-selected behavior of a written emitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmissionPolicy {
    PassThrough,
    Terminal,
}

pub type Expression<S, O, E> = Located<ExpressionKind<S, O, E>>;

/// Native expression structure. Hosts validate opaque payloads, purity and
/// supported value transitions before calling the shared runner.
#[derive(Debug, PartialEq, Eq)]
pub enum ExpressionKind<S, O, E> {
    Input,
    Binding(String),
    Group(Box<Expression<S, O, E>>),
    /// Checked native capture selection; copying retains the host value kind.
    Field {
        value: Box<Expression<S, O, E>>,
        field: Located<String>,
    },
    Sequence {
        input: Box<Expression<S, O, E>>,
        steps: Vec<Located<ExpressionStep<S, E>>>,
    },
    Set {
        operator: O,
        left: Box<Expression<S, O, E>>,
        right: Box<Expression<S, O, E>>,
    },
    Record(Vec<(Located<String>, Expression<S, O, E>)>),
}

impl<S, O, E> ExpressionKind<S, O, E> {
    // Representation-only Input/Binding/Sequence wrappers add no logical depth.
    pub(crate) fn depth(&self) -> usize {
        match self {
            Self::Group(_) | Self::Field { .. } | Self::Set { .. } | Self::Record(_) => 1,
            Self::Input | Self::Binding(_) | Self::Sequence { .. } => 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExpressionStep<S, E> {
    Stage(S),
    Emit { emitter: E, policy: EmissionPolicy },
}

/// Evaluate starts from the program's retained input. None discards the result;
/// Some requests implicit output unless the expression ends in a written emit.
#[derive(Debug, PartialEq, Eq)]
pub enum Statement<S, O, E> {
    Bind {
        name: Located<String>,
        expression: Expression<S, O, E>,
    },
    Evaluate {
        expression: Expression<S, O, E>,
        implicit_output: Option<E>,
    },
}

impl<S, O, E> Statement<S, O, E> {
    fn expression(&self) -> &Expression<S, O, E> {
        match self {
            Self::Bind { expression, .. } | Self::Evaluate { expression, .. } => expression,
        }
    }
}

/// Ordered, located statements lowered by a host frontend. Construction has no
/// effects; execution preflights all core-visible names and expression structure.
#[derive(Debug, PartialEq, Eq)]
pub struct Program<S, O, E> {
    statements: Vec<Located<Statement<S, O, E>>>,
    range: Range<usize>,
}

impl<S, O, E> Program<S, O, E> {
    pub fn new(statements: Vec<Located<Statement<S, O, E>>>, range: Range<usize>) -> Self {
        Self { statements, range }
    }

    pub fn statements(&self) -> &[Located<Statement<S, O, E>>] {
        &self.statements
    }

    /// Consume the program and transfer its located statements without copying
    /// or evaluating them. A host that retains long, flat expression spines can
    /// use this ownership to release expressions and host payloads iteratively.
    pub fn into_statements(self) -> Vec<Located<Statement<S, O, E>>> {
        self.statements
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

/// Immutable native values. Initial names are reserved for the whole program;
/// only ordered execution can add subsequent definitions. No Clone bound applies.
#[derive(Debug)]
pub struct Bindings<V> {
    values: std::sync::Arc<BTreeMap<String, std::sync::Arc<V>>>,
    locals: Vec<(String, std::sync::Arc<V>)>,
}

impl<V: PartialEq> PartialEq for Bindings<V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl<V: Eq> Eq for Bindings<V> {}

impl<V> Default for Bindings<V> {
    fn default() -> Self {
        Self {
            values: std::sync::Arc::new(BTreeMap::new()),
            locals: Vec::new(),
        }
    }
}

impl<V> Bindings<V> {
    /// Install already-admitted host values, rejecting duplicate initial names.
    pub fn new(
        initial: impl IntoIterator<Item = (Located<String>, V)>,
    ) -> Result<Self, QueryError> {
        let mut bindings = Self::default();
        for (name, value) in initial {
            if bindings.values.contains_key(&name.value) {
                return Err(QueryError::query(name.range, "binding is already defined"));
            }
            std::sync::Arc::make_mut(&mut bindings.values)
                .insert(name.value, std::sync::Arc::new(value));
        }
        Ok(bindings)
    }

    pub(crate) fn from_shared(values: BTreeMap<String, std::sync::Arc<V>>) -> Self {
        Self {
            values: std::sync::Arc::new(values),
            locals: Vec::new(),
        }
    }

    /// Iterate immutable scope entries in name order without copying values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        let mut outer = self.values.iter().peekable();
        let mut locals = self.locals.iter().peekable();
        std::iter::from_fn(move || {
            let next = match (outer.peek(), locals.peek()) {
                (Some((a, _)), Some((b, _))) if a < &b => outer.next(),
                (Some(_), Some(_)) | (None, Some(_)) => locals.next().map(|(n, v)| (n, v)),
                (Some(_), None) => outer.next(),
                (None, None) => None,
            };
            next.map(|(name, value)| (name.as_str(), value.as_ref()))
        })
    }

    /// Add a current-frame name, sharing the outer map and immutable values.
    /// Dropping the child releases its local values and restores the outer scope.
    pub fn with_local(
        &self,
        name: Located<String>,
        value: V,
        budget: &mut Budget,
    ) -> Result<Self, QueryError> {
        if self.get(&name.value, name.range.clone()).is_ok() {
            return Err(QueryError::query(name.range, "binding is already defined"));
        }
        let bytes = self
            .locals
            .iter()
            .try_fold(name.value.len() + 1, |bytes, (local, _)| {
                bytes.checked_add(local.len() + 1).ok_or_else(|| {
                    QueryError::query(name.range.clone(), "binding metadata size overflow")
                })
            })?;
        budget.charge(bytes, Location::Query(name.range.clone()))?;
        let mut locals = self
            .locals
            .iter()
            .map(|(name, value)| (name.clone(), std::sync::Arc::clone(value)))
            .collect::<Vec<_>>();
        let position = locals.partition_point(|(local, _)| local < &name.value);
        locals.insert(position, (name.value, std::sync::Arc::new(value)));
        Ok(Self {
            values: std::sync::Arc::clone(&self.values),
            locals,
        })
    }

    pub(crate) fn shared(
        &self,
        name: &str,
        range: Range<usize>,
    ) -> Result<std::sync::Arc<V>, QueryError> {
        self.locals
            .iter()
            .find(|(local, _)| local == name)
            .map(|(_, value)| value)
            .or_else(|| self.values.get(name))
            .map(std::sync::Arc::clone)
            .ok_or_else(|| QueryError::query(range, format!("undefined binding: {name}")))
    }

    /// Read a native value without copying it. The supplied query range locates
    /// a missing name, including reads inside opaque host selectors.
    pub fn get(&self, name: &str, range: Range<usize>) -> Result<&V, QueryError> {
        self.locals
            .iter()
            .find(|(local, _)| local == name)
            .map(|(_, value)| value)
            .or_else(|| self.values.get(name))
            .map(std::sync::Arc::as_ref)
            .ok_or_else(|| QueryError::query(range, format!("undefined binding: {name}")))
    }
}

/// Statically registered native operations, with the caller's context and budget.
/// Hosts validate references inside payloads, pure bindings/selector/constructor
/// operands, and stage/value applicability (including empty streams) in lowering.
/// Each producer/copy admits results and copied bytes before allocating. Hooks
/// retain native errors and may reenter evaluate_expression with relative input.
pub trait ProgramHost<C> {
    type Value;
    type Stage;
    type SetOperator;
    type Emitter;
    type Error;

    fn copy_value(
        &self,
        value: &Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>;

    fn evaluate_stage(
        &self,
        stage: &Self::Stage,
        value: Self::Value,
        context: &mut C,
        bindings: &Bindings<Self::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>;

    /// Combine left then right results using the host's native identity/order.
    fn combine(
        &self,
        operator: &Self::SetOperator,
        left: Self::Value,
        right: Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>;

    /// The core admitted the field vector's container and entries and charged
    /// copied label bytes. Moving it into a native record needs no second charge;
    /// additional host materialization must be admitted before allocation.
    fn construct_record(
        &self,
        fields: Vec<(String, Self::Value)>,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>;

    /// Borrow a native capture without converting its handles to scalar data.
    /// Hosts only unwrap their record wrapper; core lookup and copies are charged.
    fn capture_record<'v>(
        &self,
        _value: &'v Self::Value,
    ) -> Option<&'v crate::NativeCapture<Self::Value>> {
        None
    }

    /// Observe a native value at this written position. Hosts charge retained
    /// snapshots/rendered bytes and own library buffering and effect semantics.
    fn emit(
        &self,
        emitter: &Self::Emitter,
        value: &Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), EvaluationError<Self::Error>>;
}

/// Preflight the whole program, then run statements with one input/context/budget.
/// Results move into bindings; earlier statements receive charged native copies
/// of top-level input and the final statement receives the original value.
///
/// # Errors
/// Core-visible scope, structure and resource failures retain query ranges.
/// Host failures keep their native type. Emission rollback belongs to the host.
pub fn evaluate_program<C, H>(
    program: &Program<H::Stage, H::SetOperator, H::Emitter>,
    host: &H,
    input: H::Value,
    context: &mut C,
    mut bindings: Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<Bindings<H::Value>, EvaluationError<H::Error>>
where
    H: ProgramHost<C>,
{
    let mut names: BTreeSet<_> = bindings.iter().map(|(name, _)| name).collect();
    for statement in program.statements() {
        validate_expression(statement.value.expression(), &names, budget)?;
        if let Statement::Bind { name, .. } = &statement.value
            && !names.insert(&name.value)
        {
            return Err(QueryError::query(name.range.clone(), "binding is already defined").into());
        }
    }

    let mut input = Some(input);
    for (index, statement) in program.statements().iter().enumerate() {
        let statement_input = if index + 1 == program.statements().len() {
            input
                .take()
                .expect("the final statement owns retained input")
        } else {
            host.copy_value(
                input.as_ref().expect("earlier statements retain input"),
                context,
                budget,
                &statement.range,
            )?
        };
        let expression = statement.value.expression();
        let value = run_expression(
            expression,
            host,
            statement_input,
            context,
            &bindings,
            budget,
        )?;
        match &statement.value {
            Statement::Bind { name, .. } => {
                budget
                    .charge_program_work(name.value.len(), Location::Query(name.range.clone()))?;
                std::sync::Arc::make_mut(&mut bindings.values)
                    .insert(name.value.clone(), std::sync::Arc::new(value));
            }
            Statement::Evaluate {
                implicit_output, ..
            } => {
                if final_emission(expression).is_none()
                    && let Some(emitter) = implicit_output
                {
                    host.emit(emitter, &value, context, budget, &statement.range)?;
                }
            }
        }
    }
    Ok(bindings)
}

/// Run an expression with relative native input and existing immutable bindings.
/// Nested host selectors pass the same context and active budget to this function.
///
/// # Errors
/// Scope/structure preflight runs before native hooks. Runtime refusals retain
/// spent allowances while restoring active nesting and evaluation-call state.
pub fn evaluate_expression<C, H>(
    expression: &Expression<H::Stage, H::SetOperator, H::Emitter>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<H::Value, EvaluationError<H::Error>>
where
    H: ProgramHost<C>,
{
    let names = bindings.iter().map(|(name, _)| name).collect();
    validate_expression(expression, &names, budget)?;
    run_expression(expression, host, input, context, bindings, budget)
}

fn run_expression<C, H>(
    expression: &Expression<H::Stage, H::SetOperator, H::Emitter>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<H::Value, EvaluationError<H::Error>>
where
    H: ProgramHost<C>,
{
    budget.within_evaluation(
        expression.value.depth(),
        Location::Query(expression.range.clone()),
        |budget| {
            budget.charge_program_work(1, Location::Query(expression.range.clone()))?;
            match &expression.value {
                ExpressionKind::Input => Ok(input),
                ExpressionKind::Binding(name) => host.copy_value(
                    bindings.get(name, expression.range.clone())?,
                    context,
                    budget,
                    &expression.range,
                ),
                ExpressionKind::Group(child) => {
                    run_expression(child, host, input, context, bindings, budget)
                }
                ExpressionKind::Field { value, field } => {
                    let value = run_expression(value, host, input, context, bindings, budget)?;
                    let record = host.capture_record(&value).ok_or_else(|| {
                        QueryError::query(
                            field.range.clone(),
                            "field access requires a native capture",
                        )
                    })?;
                    host.copy_value(record.get(field, budget)?, context, budget, &field.range)
                }
                ExpressionKind::Sequence {
                    input: source,
                    steps,
                } => {
                    let mut value = run_expression(source, host, input, context, bindings, budget)?;
                    for step in steps {
                        match &step.value {
                            ExpressionStep::Stage(stage) => {
                                value = host.evaluate_stage(
                                    stage,
                                    value,
                                    context,
                                    bindings,
                                    budget,
                                    &step.range,
                                )?;
                            }
                            ExpressionStep::Emit { emitter, .. } => {
                                host.emit(emitter, &value, context, budget, &step.range)?;
                            }
                        }
                    }
                    Ok(value)
                }
                ExpressionKind::Set { .. } => {
                    // Consecutive left operands are flat sequencing. Retain each
                    // right input as in recursive evaluation, including copy order.
                    let mut pending = Vec::new();
                    let mut operand = expression;
                    let mut input = input;
                    while let ExpressionKind::Set {
                        operator,
                        left,
                        right,
                    } = &operand.value
                    {
                        if !pending.is_empty() {
                            budget
                                .charge_program_work(1, Location::Query(operand.range.clone()))?;
                        }
                        let left_input = host.copy_value(&input, context, budget, &left.range)?;
                        pending.push((operator, right, input, &operand.range));
                        operand = left;
                        input = left_input;
                    }
                    let mut value =
                        run_expression(operand, host, input, context, bindings, budget)?;
                    while let Some((operator, right, input, range)) = pending.pop() {
                        let right = run_expression(right, host, input, context, bindings, budget)?;
                        value = host.combine(operator, value, right, context, budget, range)?;
                    }
                    Ok(value)
                }
                ExpressionKind::Record(fields) => {
                    let units = fields.len().checked_add(1).ok_or_else(|| {
                        QueryError::query(expression.range.clone(), "result limit exceeded")
                            .with_category(QueryErrorCategory::ResultLimit)
                    })?;
                    budget.charge_results(units, Location::Query(expression.range.clone()))?;
                    let mut values = Vec::with_capacity(fields.len());
                    let mut input = Some(input);
                    for (index, (label, field)) in fields.iter().enumerate() {
                        let field_input = if index + 1 == fields.len() {
                            input.take().expect("the final field owns retained input")
                        } else {
                            host.copy_value(
                                input.as_ref().expect("earlier fields retain input"),
                                context,
                                budget,
                                &field.range,
                            )?
                        };
                        let value =
                            run_expression(field, host, field_input, context, bindings, budget)?;
                        budget.charge_program_work(
                            label.value.len(),
                            Location::Query(label.range.clone()),
                        )?;
                        values.push((label.value.clone(), value));
                    }
                    host.construct_record(values, context, budget, &expression.range)
                }
            }
        },
    )
}

fn final_emission<S, O, E>(expression: &Expression<S, O, E>) -> Option<EmissionPolicy> {
    let mut expression = expression;
    loop {
        match &expression.value {
            ExpressionKind::Group(child) => expression = child,
            ExpressionKind::Sequence { input, steps } => match steps.last() {
                Some(Located {
                    value: ExpressionStep::Emit { policy, .. },
                    ..
                }) => return Some(*policy),
                Some(_) => return None,
                None => expression = input,
            },
            ExpressionKind::Input
            | ExpressionKind::Binding(_)
            | ExpressionKind::Field { .. }
            | ExpressionKind::Set { .. }
            | ExpressionKind::Record(_) => return None,
        }
    }
}

fn validate_expression<S, O, E>(
    expression: &Expression<S, O, E>,
    names: &BTreeSet<&str>,
    budget: &Budget,
) -> Result<(), QueryError> {
    let mut pending = vec![(expression, 0, 0)];
    while let Some((expression, depth, calls)) = pending.pop() {
        let depth = depth + expression.value.depth();
        let calls = calls + 1;
        budget.check_evaluation_depth(depth, calls, Location::Query(expression.range.clone()))?;
        match &expression.value {
            ExpressionKind::Input => {}
            ExpressionKind::Binding(name) => {
                if !names.contains(name.as_str()) {
                    return Err(QueryError::query(
                        expression.range.clone(),
                        format!("undefined binding: {name}"),
                    ));
                }
            }
            ExpressionKind::Group(child) => pending.push((child, depth, calls)),
            ExpressionKind::Field { value, field } => {
                if final_emission(value) == Some(EmissionPolicy::Terminal) {
                    return Err(QueryError::query(
                        field.range.clone(),
                        "field cannot follow terminal emission",
                    ));
                }
                pending.push((value, depth, calls));
            }
            ExpressionKind::Sequence { input, steps } => {
                let mut terminal = final_emission(input) == Some(EmissionPolicy::Terminal);
                for step in steps {
                    if terminal {
                        return Err(QueryError::query(
                            step.range.clone(),
                            "a step cannot follow terminal emission",
                        ));
                    }
                    terminal = matches!(
                        step.value,
                        ExpressionStep::Emit {
                            policy: EmissionPolicy::Terminal,
                            ..
                        }
                    );
                }
                pending.push((input, depth, calls));
            }
            ExpressionKind::Set { .. } => {
                let mut operand = expression;
                while let ExpressionKind::Set { left, right, .. } = &operand.value {
                    budget.check_evaluation_depth(
                        depth,
                        calls,
                        Location::Query(operand.range.clone()),
                    )?;
                    pending.push((right, depth, calls));
                    operand = left;
                }
                pending.push((operand, depth, calls));
            }
            ExpressionKind::Record(fields) => {
                pending.extend(fields.iter().rev().map(|(_, field)| (field, depth, calls)));
            }
        }
    }
    Ok(())
}

/// Release a retained native tree without recursively dropping its flat set spine.
/// Hosts owning normalized operands use this when their immutable tree is released.
pub fn dispose_expression<S, O, E>(expression: Expression<S, O, E>) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match expression.value {
            ExpressionKind::Input | ExpressionKind::Binding(_) => {}
            ExpressionKind::Group(child) => pending.push(*child),
            ExpressionKind::Field { value, .. } => pending.push(*value),
            ExpressionKind::Sequence { input, .. } => pending.push(*input),
            ExpressionKind::Set { left, right, .. } => {
                pending.push(*left);
                pending.push(*right);
            }
            ExpressionKind::Record(fields) => {
                pending.extend(fields.into_iter().map(|(_, expression)| expression))
            }
        }
    }
}
