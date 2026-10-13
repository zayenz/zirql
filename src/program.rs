use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::{Budget, EvaluationError, Located, Location, QueryError};

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
    fn depth(&self) -> usize {
        match self {
            Self::Group(_) | Self::Set { .. } | Self::Record(_) => 1,
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
#[derive(Debug, PartialEq, Eq)]
pub struct Bindings<V> {
    values: BTreeMap<String, V>,
}

impl<V> Default for Bindings<V> {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
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
            bindings.values.insert(name.value, value);
        }
        Ok(bindings)
    }

    /// Read a native value without copying it. The supplied query range locates
    /// a missing name, including reads inside opaque host selectors.
    pub fn get(&self, name: &str, range: Range<usize>) -> Result<&V, QueryError> {
        self.values
            .get(name)
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
    let mut names: BTreeSet<_> = bindings.values.keys().map(String::as_str).collect();
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
                bindings.values.insert(name.value.clone(), value);
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
    let names = bindings.values.keys().map(String::as_str).collect();
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
