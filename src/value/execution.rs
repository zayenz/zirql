//! One value runner. User calls, operands and generators suspend in flat vectors.
use super::prepared::ReadSlot;
use super::{
    Budget, ConstructedValue, DeclaredType, Frame, KeyAxis, KeyedEntry, Located, Location,
    PreparedArgument, PreparedArguments, PreparedGenerator, PreparedKind, PreparedValue,
    QueryError, RuntimeValue, SetElementType, ValueBinaryOperator, ValueConstructor, ValueKey,
    ValueQueryHost, admit_container, admit_diagnostic, allocation, annotate, binary, boolean,
    constructed_value_type, copy_axes, copy_constructed_value, copy_declaration, declaration,
    entries, equal_keys, error, expose, finish_keyed, group_by, insert_set, integer, integer_range,
    key_positions, key_set_type, key_value, metadata_vec, optional_access, push_metadata,
    record_declaration, reduce_ref, scalar, select_field, source_axis, sum, take_one,
    take_position, unary, underlying, value_keys, zip_exact,
};
use std::rc::Rc;

pub(crate) struct Body<'a, Q, K> {
    pub declaration: crate::FunctionDeclarationRef<'a, Q, K>,
    pub bindings: Vec<(&'a str, PreparedValue<'a, Q>)>,
    pub value: PreparedValue<'a, Q>,
}

pub(crate) enum Data<N> {
    Constructed(RuntimeValue),
    Native(std::sync::Arc<N>),
}
pub(crate) struct Value<'a, N, K> {
    pub data: Data<N>,
    pub contract: Option<&'a DeclaredType<K>>,
}
impl<'a, N, K> Value<'a, N, K> {
    pub fn constructed(value: RuntimeValue) -> Self {
        Self {
            data: Data::Constructed(value),
            contract: None,
        }
    }
    pub fn share(&mut self, budget: &mut Budget, location: &Location) -> Result<Self, QueryError> {
        let data = match &mut self.data {
            Data::Constructed(value) => Data::Constructed(value.share(budget, location)?),
            Data::Native(value) => {
                budget.charge(1, location.clone())?;
                Data::Native(std::sync::Arc::clone(value))
            }
        };
        Ok(Self {
            data,
            contract: self.contract,
        })
    }
    fn scalar(&self, location: &Location) -> Result<&ConstructedValue, QueryError> {
        match &self.data {
            Data::Constructed(value) => Ok(value.as_ref()),
            Data::Native(_) => Err(error(
                location,
                "native handles require an explicit scalar projection",
            )),
        }
    }
    pub fn materialize(
        self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<ConstructedValue, QueryError> {
        match self.data {
            Data::Constructed(value) => {
                let value = value.materialize(budget, location)?;
                if let Some(contract) = self.contract {
                    if crate::runtime_types::contains_native(contract, budget, location)? {
                        Ok(value)
                    } else {
                        annotate(
                            value,
                            crate::runtime_types::constructed_declaration(
                                contract, budget, location,
                            )?,
                            budget,
                            location,
                        )
                    }
                } else {
                    Ok(value)
                }
            }
            Data::Native(_) => Err(error(
                location,
                "native handles require an explicit scalar projection",
            )),
        }
    }
}

type Slots<'a, N, K> = [(&'a str, Value<'a, N, K>)];
pub(crate) trait Host<'a, Q> {
    type Native;
    type Kind: PartialEq;
    type Error: From<QueryError>;
    fn outer(
        &mut self,
        slot: usize,
        name: &str,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error>;
    #[expect(
        clippy::too_many_arguments,
        reason = "A native callback borrows the active request, explicit scope and argument continuations"
    )]
    fn native(
        &mut self,
        query: &'a Q,
        arguments: &'a [PreparedArgument<'a, Q>],
        locals: &mut Slots<'a, Self::Native, Self::Kind>,
        bodies: &'a [Body<'a, Q, Self::Kind>],
        depth: usize,
        function: Option<usize>,
        projected: bool,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error>;
    fn admit_native(
        &mut self,
        value: &Self::Native,
        expected: &DeclaredType<Self::Kind>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), Self::Error>;
    fn subjects(
        &mut self,
        _value: std::sync::Arc<Self::Native>,
        _budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<Self::Native>, Self::Error> {
        Err(error(location, "in source needs an array or supported set").into())
    }
    fn check(
        &mut self,
        value: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError>;
    fn failure(&mut self, _calls: &[(usize, std::ops::Range<usize>)], _omitted: usize) {}
}

struct Activation<'a, N, K> {
    function: Option<usize>,
    range: std::ops::Range<usize>,
    base: usize,
    locals: Vec<(&'a str, Value<'a, N, K>)>,
    names: &'a [&'a str],
}
type ResultValue<'a, H, Q> = Value<'a, <H as Host<'a, Q>>::Native, <H as Host<'a, Q>>::Kind>;

enum Pending<'a, Q, N, K> {
    Eval(&'a PreparedValue<'a, Q>, usize, bool),
    Check(&'a PreparedValue<'a, Q>, usize),
    Apply(
        &'a PreparedValue<'a, Q>,
        usize,
        Vec<&'a PreparedValue<'a, Q>>,
        Vec<Value<'a, N, K>>,
    ),
    Branch(&'a PreparedValue<'a, Q>, usize, usize, bool),
    Require(&'a PreparedValue<'a, Q>, usize, bool),
    Default(&'a PreparedValue<'a, Q>, usize, bool),
    Fallback(&'a PreparedValue<'a, Q>, usize, Value<'a, N, K>),
    Call(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        bool,
        Vec<Value<'a, N, K>>,
    ),
    Set(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Vec<ConstructedValue>,
        Option<SetElementType>,
    ),
    KeyedKey(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Option<Vec<KeyAxis>>,
        Vec<Located<KeyedEntry>>,
    ),
    KeyedValue(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Option<Vec<KeyAxis>>,
        Vec<Located<KeyedEntry>>,
        Vec<ValueKey>,
    ),
    Let(usize, usize),
    Return(usize),
    Clauses(Rc<Comprehension<'a, Q>>, usize, usize),
    Source(Rc<Comprehension<'a, Q>>, usize, usize),
    Candidate(
        Rc<Comprehension<'a, Q>>,
        usize,
        usize,
        GeneratorSource<'a, N, K>,
        Vec<usize>,
        usize,
    ),
    Filter(Rc<Comprehension<'a, Q>>, usize, usize),
    Restore(usize),
    Head(Rc<Comprehension<'a, Q>>),
    KeyHead(Rc<Comprehension<'a, Q>>, Vec<ValueKey>),
    FinishComprehension(Rc<Comprehension<'a, Q>>),
    EmptyAxes(Rc<Comprehension<'a, Q>>, usize, Vec<KeyAxis>),
}
enum GeneratorSource<'a, N, K> {
    Constructed(RuntimeValue),
    Native(Vec<Value<'a, N, K>>),
}
impl<'a, N, K> GeneratorSource<'a, N, K> {
    fn len(&self) -> usize {
        match self {
            Self::Constructed(value) => match value.as_ref() {
                ConstructedValue::Array(values)
                | ConstructedValue::Set {
                    elements: values, ..
                } => values.len(),
                ConstructedValue::KeyedArray { entries, .. } => entries.len(),
                _ => unreachable!("admitted generator source"),
            },
            Self::Native(values) => values.len(),
        }
    }
    fn child(
        &mut self,
        position: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, N, K>, QueryError> {
        match self {
            Self::Constructed(value) => Ok(Value::constructed(
                value
                    .share(budget, location)?
                    .child(position, budget, location)?,
            )),
            Self::Native(values) => values[position].share(budget, location),
        }
    }
}
struct Comprehension<'a, Q> {
    expression: &'a PreparedValue<'a, Q>,
    clauses: &'a [Located<PreparedGenerator<'a, Q>>],
    key: Option<&'a PreparedValue<'a, Q>>,
    head: &'a PreparedValue<'a, Q>,
    empty: Option<&'a [PreparedValue<'a, Q>]>,
    depth: usize,
    output: std::cell::RefCell<Output>,
}
enum Output {
    Array(Vec<ConstructedValue>),
    Set(Vec<ConstructedValue>, Option<SetElementType>),
    Keyed(Vec<Located<KeyedEntry>>),
}

pub(crate) fn execute<'a, Q, H: Host<'a, Q>>(
    entry: &'a PreparedValue<'a, Q>,
    names: &'a [&'a str],
    bodies: &'a [Body<'a, Q, H::Kind>],
    host: &mut H,
    budget: &mut Budget,
    limit: usize,
    offset: usize,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let location = Location::Query(entry.range.clone());
    budget.within_evaluation(0, location.clone(), |budget| {
        let mut pending = metadata_vec(1, budget, &location)?;
        pending.push(Pending::Eval(entry, 1, false));
        let mut frames = metadata_vec(1, budget, &location)?;
        frames.push(Activation {
            function: None,
            range: entry.range.clone(),
            base: 0,
            locals: Vec::new(),
            names,
        });
        let mut result: Option<ResultValue<'a, H, Q>> = None;
        let outcome = (|| {
            while let Some(next) = pending.pop() {
                let frame_depth = frames.len();
                let frame = frames.last().expect("active expression scope");
                let location = match &next {
                    Pending::Eval(e, ..)
                    | Pending::Check(e, ..)
                    | Pending::Apply(e, ..)
                    | Pending::Branch(e, ..)
                    | Pending::Require(e, ..)
                    | Pending::Default(e, ..)
                    | Pending::Fallback(e, ..)
                    | Pending::Call(e, ..)
                    | Pending::Set(e, ..)
                    | Pending::KeyedKey(e, ..)
                    | Pending::KeyedValue(e, ..) => Location::Query(e.range.clone()),
                    _ => Location::Query(frame.range.clone()),
                };
                let depth = match &next {
                    Pending::Eval(_, depth, _)
                    | Pending::Check(_, depth)
                    | Pending::Apply(_, depth, ..)
                    | Pending::Branch(_, depth, ..)
                    | Pending::Require(_, depth, _)
                    | Pending::Default(_, depth, _)
                    | Pending::Fallback(_, depth, _)
                    | Pending::Call(_, depth, ..)
                    | Pending::Set(_, depth, ..)
                    | Pending::KeyedKey(_, depth, ..)
                    | Pending::KeyedValue(_, depth, ..) => *depth,
                    Pending::Clauses(c, index, _)
                    | Pending::Source(c, index, _)
                    | Pending::Candidate(c, index, ..)
                    | Pending::Filter(c, index, _) => c.depth + index + 1,
                    Pending::Head(c) | Pending::KeyHead(c, _) => c.depth + c.clauses.len(),
                    Pending::FinishComprehension(c) | Pending::EmptyAxes(c, ..) => c.depth,
                    Pending::Let(..) | Pending::Return(_) | Pending::Restore(_) => 0,
                };
                budget.within_evaluation(
                    depth,
                    location.clone(),
                    |budget| -> Result<(), H::Error> {
                        let frame = frames.last_mut().expect("active expression scope");
                        // Each push is admitted before vector growth. Stored pending work is
                        // bounded by spending; completed/tail activations leave no history.
                        macro_rules! push {
                            ($v:expr) => {
                                push_metadata(&mut pending, $v, budget, &location)?
                            };
                        }
                        macro_rules! owned {
                            ($v:expr) => {
                                Some(Value::constructed(RuntimeValue::Owned($v)))
                            };
                        }
                        match next {
                            Pending::Eval(expression, depth, tail) => {
                                budget.charge(1, location.clone())?;
                                push!(Pending::Check(expression, depth));
                                match &expression.value {
                                    PreparedKind::Literal(value) => {
                                        result =
                                            owned!(copy_constructed_value(value, budget, location)?)
                                    }
                                    PreparedKind::Name(ReadSlot::Local(index)) => {
                                        result =
                                            Some(frame.locals[*index].1.share(budget, &location)?)
                                    }
                                    PreparedKind::Name(ReadSlot::Outer(index)) => {
                                        result = Some(host.outer(
                                            *index,
                                            frame.names[*index],
                                            budget,
                                            &expression.range,
                                        )?)
                                    }
                                    PreparedKind::NativeQuery { query, arguments }
                                    | PreparedKind::NativeValue { query, arguments } => {
                                        result = Some(host.native(
                                            query,
                                            arguments,
                                            &mut frame.locals,
                                            bodies,
                                            offset + frame_depth - 1,
                                            frame.function,
                                            matches!(
                                                expression.value,
                                                PreparedKind::NativeQuery { .. }
                                            ),
                                            budget,
                                            &expression.range,
                                        )?);
                                    }
                                    PreparedKind::Call { arguments, .. } => {
                                        let values =
                                            metadata_vec(arguments.len(), budget, &location)?;
                                        push!(Pending::Call(expression, depth, 0, tail, values));
                                        if let Some(first) = arguments.first() {
                                            push!(Pending::Eval(first, depth + 1, false));
                                        }
                                    }
                                    PreparedKind::Conditional {
                                        branches,
                                        otherwise,
                                    } => {
                                        if let Some((condition, _)) = branches.first() {
                                            push!(Pending::Branch(expression, depth, 0, tail));
                                            push!(Pending::Eval(condition, depth + 1, false));
                                        } else {
                                            push!(Pending::Eval(otherwise, depth + 1, tail));
                                        }
                                    }
                                    PreparedKind::Require { condition, .. } => {
                                        push!(Pending::Require(expression, depth, tail));
                                        push!(Pending::Eval(condition, depth + 1, false));
                                    }
                                    PreparedKind::Default { value, .. } => {
                                        push!(Pending::Default(expression, depth, tail));
                                        push!(Pending::Eval(value, depth + 1, false));
                                    }
                                    PreparedKind::Set(children) => {
                                        let values =
                                            admit_container(children.len(), budget, &location)?;
                                        if let Some(child) = children.first() {
                                            push!(Pending::Set(expression, depth, 0, values, None));
                                            push!(Pending::Eval(child, depth + 1, false));
                                        } else {
                                            result = owned!(ConstructedValue::Set {
                                                element_type: None,
                                                elements: values
                                            });
                                        }
                                    }
                                    PreparedKind::KeyedArray { axes, entries } => {
                                        let axes = axes
                                            .map(|axes| copy_axes(axes, budget, &location))
                                            .transpose()?;
                                        let values =
                                            admit_container(entries.len(), budget, &location)?;
                                        if let Some((key, _)) = entries.first() {
                                            push!(Pending::KeyedKey(
                                                expression, depth, 0, axes, values
                                            ));
                                            push!(Pending::Eval(key, depth + 1, false));
                                        } else {
                                            result = owned!(finish_keyed(
                                                axes, values, budget, &location
                                            )?);
                                        }
                                    }
                                    PreparedKind::Comprehension {
                                        constructor,
                                        clauses,
                                        head,
                                    } => {
                                        let output = match constructor {
                                            ValueConstructor::Array => Output::Array(
                                                admit_container(0, budget, &location)?,
                                            ),
                                            ValueConstructor::Set => Output::Set(
                                                admit_container(0, budget, &location)?,
                                                None,
                                            ),
                                        };
                                        let c = Rc::new(Comprehension {
                                            expression,
                                            clauses,
                                            key: None,
                                            head,
                                            empty: None,
                                            depth,
                                            output: std::cell::RefCell::new(output),
                                        });
                                        push!(Pending::FinishComprehension(Rc::clone(&c)));
                                        push!(Pending::Clauses(c, 0, frame.locals.len()));
                                    }
                                    PreparedKind::KeyedComprehension {
                                        clauses,
                                        key,
                                        head,
                                        empty_axis_sources,
                                    } => {
                                        let c = Rc::new(Comprehension {
                                            expression,
                                            clauses,
                                            key: Some(key),
                                            head,
                                            empty: empty_axis_sources.as_deref(),
                                            depth,
                                            output: std::cell::RefCell::new(Output::Keyed(
                                                admit_container(0, budget, &location)?,
                                            )),
                                        });
                                        push!(Pending::FinishComprehension(Rc::clone(&c)));
                                        push!(Pending::Clauses(c, 0, frame.locals.len()));
                                    }
                                    _ => {
                                        let children = operands(expression, budget, &location)?;
                                        let values =
                                            metadata_vec(children.len(), budget, &location)?;
                                        let first = children.first().copied();
                                        push!(Pending::Apply(expression, depth, children, values));
                                        if let Some(first) = first {
                                            push!(Pending::Eval(first, depth + 1, false));
                                        }
                                    }
                                }
                            }
                            Pending::Check(expression, _) => {
                                if let Some(Value {
                                    data: Data::Constructed(value),
                                    ..
                                }) = &result
                                {
                                    host.check(
                                        value.as_ref(),
                                        budget,
                                        &Location::Query(expression.range.clone()),
                                    )?;
                                }
                            }
                            Pending::Apply(expression, depth, children, mut values) => {
                                if !children.is_empty() {
                                    let value = result.take().expect("operand result");
                                    let shared = matches!(
                                        expression.value,
                                        PreparedKind::Field { .. }
                                            | PreparedKind::Occurs(_)
                                            | PreparedKind::Absent(_)
                                            | PreparedKind::Reduce { .. }
                                    );
                                    let value = if values.is_empty()
                                        && matches!(expression.value, PreparedKind::Range { .. })
                                    {
                                        Value::constructed(RuntimeValue::Owned(
                                            ConstructedValue::Integer(integer(
                                                value.materialize(budget, &location)?,
                                                &location,
                                            )?),
                                        ))
                                    } else if values.is_empty()
                                        && matches!(
                                            expression.value,
                                            PreparedKind::ArrayAccess { .. }
                                        )
                                    {
                                        Value::constructed(RuntimeValue::Owned(expose(
                                            value.materialize(budget, &location)?,
                                            budget,
                                            &location,
                                        )?))
                                    } else {
                                        value
                                    };
                                    values.push(if shared {
                                        value
                                    } else {
                                        Value::constructed(RuntimeValue::Owned(
                                            value.materialize(budget, &location)?,
                                        ))
                                    });
                                }
                                if values.len() == 1
                                    && let PreparedKind::Binary {
                                        operator:
                                            operator @ (ValueBinaryOperator::And
                                            | ValueBinaryOperator::Or),
                                        ..
                                    } = expression.value
                                {
                                    let a = match values[0].scalar(&location)?.data() {
                                        ConstructedValue::Boolean(a) => *a,
                                        _ => {
                                            return Err(error(
                                                &location,
                                                "condition needs a fixed Boolean",
                                            )
                                            .into());
                                        }
                                    };
                                    if (operator == ValueBinaryOperator::And && !a)
                                        || (operator == ValueBinaryOperator::Or && a)
                                    {
                                        result = owned!(scalar(
                                            ConstructedValue::Boolean(a),
                                            budget,
                                            &location
                                        )?);
                                        return Ok(());
                                    }
                                }
                                if values.len() < children.len() {
                                    let next = children[values.len()];
                                    push!(Pending::Apply(expression, depth, children, values));
                                    push!(Pending::Eval(next, depth + 1, false));
                                } else {
                                    result = Some(apply(expression, values, budget)?);
                                }
                            }
                            Pending::Branch(expression, depth, index, tail) => {
                                let PreparedKind::Conditional {
                                    branches,
                                    otherwise,
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if boolean(
                                    result
                                        .take()
                                        .expect("condition")
                                        .materialize(budget, &location)?,
                                    &Location::Query(branches[index].0.range.clone()),
                                )? {
                                    push!(Pending::Eval(&branches[index].1, depth + 1, tail));
                                } else if let Some((condition, _)) = branches.get(index + 1) {
                                    push!(Pending::Branch(expression, depth, index + 1, tail));
                                    push!(Pending::Eval(condition, depth + 1, false));
                                } else {
                                    push!(Pending::Eval(otherwise, depth + 1, tail));
                                }
                            }
                            Pending::Require(expression, depth, tail) => {
                                let PreparedKind::Require {
                                    condition,
                                    value,
                                    message,
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if !boolean(
                                    result
                                        .take()
                                        .expect("require condition")
                                        .materialize(budget, &location)?,
                                    &Location::Query(condition.range.clone()),
                                )? {
                                    admit_diagnostic("", &message.value, budget, &location)?;
                                    return Err(error(&location, message.value.clone()).into());
                                }
                                push!(Pending::Eval(value, depth + 1, tail));
                            }
                            Pending::Default(expression, depth, tail) => {
                                let left = result.take().expect("default left");
                                if matches!(&left.data, Data::Native(_))
                                    || left.scalar(&location)?.occurs()
                                {
                                    result = Some(left);
                                } else {
                                    let PreparedKind::Default { fallback, .. } = &expression.value
                                    else {
                                        unreachable!()
                                    };
                                    let typed = left.contract.is_some()
                                        || declaration(left.scalar(&location)?).is_some();
                                    if typed {
                                        push!(Pending::Fallback(expression, depth, left));
                                    }
                                    push!(Pending::Eval(fallback, depth + 1, tail && !typed));
                                }
                            }
                            Pending::Fallback(_, _, left) => {
                                let right = result.take().expect("default fallback");
                                if let Some(mut expected) = left.contract {
                                    if matches!(&right.data, Data::Native(_))
                                        || right.scalar(&location)?.occurs()
                                    {
                                        while let DeclaredType::Optional(child) = expected {
                                            expected = child;
                                        }
                                    }
                                    let mut right = right;
                                    admit(&mut right, expected, host, budget, &location)?;
                                    result = Some(right);
                                    return Ok(());
                                }
                                let expected = match left.contract {
                                    Some(c) => crate::runtime_types::constructed_declaration(
                                        c, budget, &location,
                                    )?,
                                    None => copy_declaration(
                                        declaration(left.scalar(&location)?).expect("typed absent"),
                                        budget,
                                        &location,
                                    )?,
                                };
                                let expected = if right.scalar(&location)?.occurs() {
                                    underlying(&expected)
                                } else {
                                    &expected
                                };
                                crate::admit_constructed(
                                    right.scalar(&location)?,
                                    expected,
                                    budget,
                                    location.clone(),
                                )?;
                                result = owned!(annotate(
                                    right.materialize(budget, &location)?,
                                    copy_declaration(expected, budget, &location)?,
                                    budget,
                                    &location
                                )?);
                            }
                            Pending::Call(expression, depth, index, tail, mut arguments) => {
                                let PreparedKind::Call {
                                    function,
                                    arguments: sources,
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if !sources.is_empty() {
                                    arguments.push(result.take().expect("call argument"));
                                }
                                if let Some(next) = sources.get(index + 1) {
                                    push!(Pending::Call(
                                        expression,
                                        depth,
                                        index + 1,
                                        tail,
                                        arguments
                                    ));
                                    push!(Pending::Eval(next, depth + 1, false));
                                    return Ok(());
                                }
                                budget.charge(1, location.clone())?;
                                let body = bodies.get(*function).ok_or_else(|| {
                                    error(&location, "function was not prepared in this request")
                                })?;
                                // All arguments still refer to old slots until admission completes.
                                for ((argument, parameter), source) in arguments
                                    .iter_mut()
                                    .zip(body.declaration.parameters)
                                    .zip(sources)
                                {
                                    admit(
                                        argument,
                                        &parameter.value.declaration,
                                        host,
                                        budget,
                                        &Location::Query(source.range.clone()),
                                    )?;
                                }
                                let reuse = if tail {
                                    match frame.function {
                                        Some(caller) => crate::runtime_types::same_declaration(
                                            &bodies[caller].declaration.result.value,
                                            &body.declaration.result.value,
                                            budget,
                                            &location,
                                        )?,
                                        None => false,
                                    }
                                } else {
                                    false
                                };
                                if reuse {
                                    pending.truncate(frame.base);
                                    // Arguments retain any old-slot aliases until admission has
                                    // finished; installing them then releases dead locals.
                                    frame.locals.clear();
                                    if frame.locals.capacity()
                                        < body.declaration.parameters.len() + body.bindings.len()
                                    {
                                        budget.charge(
                                            body.declaration.parameters.len() + body.bindings.len(),
                                            location.clone(),
                                        )?;
                                        frame.locals.reserve(
                                            body.declaration.parameters.len() + body.bindings.len(),
                                        );
                                    }
                                    frame.locals.extend(
                                        body.declaration
                                            .parameters
                                            .iter()
                                            .zip(arguments)
                                            .map(|(p, v)| (p.value.name.value.as_str(), v)),
                                    );
                                    frame.function = Some(*function);
                                    frame.range = expression.range.clone();
                                } else {
                                    if offset + frames.len() > limit {
                                        return Err(error(
                                            &location,
                                            "active function depth limit exceeded",
                                        )
                                        .into());
                                    }
                                    let mut locals = metadata_vec(
                                        body.declaration.parameters.len() + body.bindings.len(),
                                        budget,
                                        &location,
                                    )?;
                                    locals.extend(
                                        body.declaration
                                            .parameters
                                            .iter()
                                            .zip(arguments)
                                            .map(|(p, v)| (p.value.name.value.as_str(), v)),
                                    );
                                    let base = pending.len();
                                    push_metadata(
                                        &mut frames,
                                        Activation {
                                            function: Some(*function),
                                            range: expression.range.clone(),
                                            base,
                                            locals,
                                            names: &[],
                                        },
                                        budget,
                                        &location,
                                    )?;
                                }
                                push!(Pending::Return(*function));
                                push!(Pending::Let(*function, 0));
                            }
                            Pending::Set(expression, depth, index, mut values, mut ty) => {
                                let value = result
                                    .take()
                                    .expect("set child")
                                    .materialize(budget, &location)?;
                                insert_set(&mut values, &mut ty, value, false, budget, &location)?;
                                let PreparedKind::Set(children) = &expression.value else {
                                    unreachable!()
                                };
                                if let Some(child) = children.get(index + 1) {
                                    push!(Pending::Set(expression, depth, index + 1, values, ty));
                                    push!(Pending::Eval(child, depth + 1, false));
                                } else {
                                    result = owned!(ConstructedValue::Set {
                                        element_type: ty,
                                        elements: values
                                    });
                                }
                            }
                            Pending::KeyedKey(expression, depth, index, axes, values) => {
                                let PreparedKind::KeyedArray { entries, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let key = result
                                    .take()
                                    .expect("literal key")
                                    .materialize(budget, &location)?;
                                let keys = value_keys(
                                    key,
                                    budget,
                                    &Location::Query(entries[index].0.range.clone()),
                                )?;
                                push!(Pending::KeyedValue(
                                    expression, depth, index, axes, values, keys
                                ));
                                push!(Pending::Eval(&entries[index].1, depth + 1, false));
                            }
                            Pending::KeyedValue(
                                expression,
                                depth,
                                index,
                                axes,
                                mut values,
                                keys,
                            ) => {
                                let PreparedKind::KeyedArray { entries, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let value = result
                                    .take()
                                    .expect("literal cell")
                                    .materialize(budget, &location)?;
                                values.push(Located::new(
                                    KeyedEntry { key: keys, value },
                                    entries[index].0.range.clone(),
                                ));
                                if let Some((key, _)) = entries.get(index + 1) {
                                    push!(Pending::KeyedKey(
                                        expression,
                                        depth,
                                        index + 1,
                                        axes,
                                        values
                                    ));
                                    push!(Pending::Eval(key, depth + 1, false));
                                } else {
                                    result = owned!(finish_keyed(axes, values, budget, &location)?);
                                }
                            }
                            Pending::Let(function, index) => {
                                let body = &bodies[function];
                                if index > 0 {
                                    frame.locals.push((
                                        body.bindings[index - 1].0,
                                        result.take().expect("local let"),
                                    ));
                                }
                                if let Some((_, expression)) = body.bindings.get(index) {
                                    push!(Pending::Let(function, index + 1));
                                    push!(Pending::Eval(expression, 1, false));
                                } else {
                                    push!(Pending::Eval(&body.value, 1, true));
                                }
                            }
                            Pending::Return(function) => {
                                let mut value = result.take().expect("function result");
                                admit(
                                    &mut value,
                                    &bodies[function].declaration.result.value,
                                    host,
                                    budget,
                                    &Location::Query(bodies[function].value.range.clone()),
                                )?;
                                frames.pop();
                                result = Some(value);
                            }
                            Pending::Clauses(c, index, saved) => {
                                if let Some(clause) = c.clauses.get(index) {
                                    budget.charge(1, Location::Query(clause.range.clone()))?;
                                    push!(Pending::Source(Rc::clone(&c), index, saved));
                                    let source = match &clause.value {
                                        PreparedGenerator::Bind { value, .. } => value,
                                        PreparedGenerator::In { source, .. } => source,
                                    };
                                    push!(Pending::Eval(source, c.depth + index + 2, false));
                                } else {
                                    push!(Pending::Head(Rc::clone(&c)));
                                    push!(Pending::Eval(
                                        c.key.unwrap_or(c.head),
                                        c.depth + index + 1,
                                        false
                                    ));
                                }
                            }
                            Pending::Source(c, index, saved) => {
                                let value = result.take().expect("generator source");
                                let clause = &c.clauses[index];
                                let cl = Location::Query(clause.range.clone());
                                match &clause.value {
                                    PreparedGenerator::Bind { binder, filter, .. } => {
                                        budget.charge(1, cl.clone())?;
                                        if let Some(name) = &binder.value {
                                            push_metadata(
                                                &mut frame.locals,
                                                (name, value),
                                                budget,
                                                &cl,
                                            )?;
                                        }
                                        push!(Pending::Restore(saved));
                                        if let Some(filter) = filter {
                                            push!(Pending::Filter(
                                                Rc::clone(&c),
                                                index,
                                                frame.locals.len()
                                            ));
                                            push!(Pending::Eval(
                                                filter,
                                                c.depth + index + 2,
                                                false
                                            ));
                                        } else {
                                            push!(Pending::Clauses(
                                                c,
                                                index + 1,
                                                frame.locals.len()
                                            ));
                                        }
                                    }
                                    PreparedGenerator::In { binders, .. } => {
                                        let contract = value.contract;
                                        let value = match value.data {
                                            Data::Native(owner) => {
                                                let values = host.subjects(owner, budget, &cl)?;
                                                budget.check_items(values.len(), cl.clone())?;
                                                let mut items =
                                                    metadata_vec(values.len(), budget, &cl)?;
                                                let element =
                                                    contract.and_then(|contract| match contract {
                                                        DeclaredType::Sequence(element)
                                                        | DeclaredType::Array(element) => {
                                                            Some(element.as_ref())
                                                        }
                                                        _ => None,
                                                    });
                                                for value in values {
                                                    budget.charge(1, cl.clone())?;
                                                    budget.charge_results(1, cl.clone())?;
                                                    items.push(Value {
                                                        data: Data::Native(std::sync::Arc::new(
                                                            value,
                                                        )),
                                                        contract: element,
                                                    });
                                                }
                                                GeneratorSource::Native(items)
                                            }
                                            Data::Constructed(data) => {
                                                let value = Value {
                                                    data: Data::<H::Native>::Constructed(data),
                                                    contract,
                                                };
                                                let value = if contract.is_some() {
                                                    RuntimeValue::Owned(
                                                        value.materialize(budget, &cl)?,
                                                    )
                                                } else {
                                                    let Data::Constructed(value) = value.data
                                                    else {
                                                        unreachable!()
                                                    };
                                                    value
                                                };
                                                let value = if matches!(
                                                    value.as_ref(),
                                                    ConstructedValue::Declared { .. }
                                                ) {
                                                    RuntimeValue::Owned(expose(
                                                        value.materialize(budget, &cl)?,
                                                        budget,
                                                        &cl,
                                                    )?)
                                                } else {
                                                    value
                                                };
                                                if !matches!(
                                                    value.as_ref(),
                                                    ConstructedValue::Array(_)
                                                        | ConstructedValue::Set { .. }
                                                        | ConstructedValue::KeyedArray { .. }
                                                ) {
                                                    return Err(error(
                                                        &cl,
                                                        "in source needs an array or supported set",
                                                    )
                                                    .into());
                                                }
                                                GeneratorSource::Constructed(value)
                                            }
                                        };
                                        let len = value.len();
                                        let mut count = 1usize;
                                        for _ in binders {
                                            budget.charge(1, cl.clone())?;
                                            count = count.checked_mul(len).ok_or_else(|| {
                                                error(&cl, "grouped generator cardinality overflow")
                                            })?;
                                        }
                                        budget.charge(count, cl.clone())?;
                                        let mut positions =
                                            metadata_vec(binders.len(), budget, &cl)?;
                                        positions.resize(binders.len(), 0);
                                        push!(Pending::Candidate(
                                            c, index, saved, value, positions, count
                                        ));
                                    }
                                }
                            }
                            Pending::Candidate(
                                c,
                                index,
                                saved,
                                mut source,
                                mut positions,
                                count,
                            ) => {
                                frame.locals.truncate(saved);
                                if count == 0 {
                                    return Ok(());
                                }
                                let clause = &c.clauses[index];
                                let PreparedGenerator::In {
                                    binders, filter, ..
                                } = &clause.value
                                else {
                                    unreachable!()
                                };
                                let cl = Location::Query(clause.range.clone());
                                let len = source.len();
                                for (binder, position) in binders.iter().zip(&positions) {
                                    budget.charge(1, Location::Query(binder.range.clone()))?;
                                    if let Some(name) = &binder.value {
                                        let v = source.child(*position, budget, &cl)?;
                                        push_metadata(&mut frame.locals, (name, v), budget, &cl)?;
                                    }
                                }
                                for position in positions.iter_mut().rev() {
                                    budget.charge(1, cl.clone())?;
                                    *position += 1;
                                    if *position < len {
                                        break;
                                    }
                                    *position = 0;
                                }
                                push!(Pending::Candidate(
                                    Rc::clone(&c),
                                    index,
                                    saved,
                                    source,
                                    positions,
                                    count - 1
                                ));
                                if let Some(filter) = filter {
                                    push!(Pending::Filter(
                                        Rc::clone(&c),
                                        index,
                                        frame.locals.len()
                                    ));
                                    push!(Pending::Eval(filter, c.depth + index + 2, false));
                                } else {
                                    push!(Pending::Clauses(c, index + 1, frame.locals.len()));
                                }
                            }
                            Pending::Filter(c, index, saved) => {
                                let filter = match &c.clauses[index].value {
                                    PreparedGenerator::Bind { filter, .. }
                                    | PreparedGenerator::In { filter, .. } => {
                                        filter.as_ref().expect("generator filter")
                                    }
                                };
                                if boolean(
                                    result
                                        .take()
                                        .expect("filter")
                                        .materialize(budget, &location)?,
                                    &Location::Query(filter.range.clone()),
                                )? {
                                    push!(Pending::Clauses(c, index + 1, saved));
                                }
                            }
                            Pending::Restore(saved) => frame.locals.truncate(saved),
                            Pending::Head(c) => {
                                let value = result
                                    .take()
                                    .expect("head or key")
                                    .materialize(budget, &location)?;
                                if let Some(key) = c.key {
                                    let keys = value_keys(
                                        value,
                                        budget,
                                        &Location::Query(key.range.clone()),
                                    )?;
                                    push!(Pending::KeyHead(Rc::clone(&c), keys));
                                    push!(Pending::Eval(
                                        c.head,
                                        c.depth + c.clauses.len() + 1,
                                        false
                                    ));
                                } else {
                                    append(&c, value, None, budget)?;
                                }
                            }
                            Pending::KeyHead(c, key) => {
                                let value = result
                                    .take()
                                    .expect("keyed head")
                                    .materialize(budget, &location)?;
                                append(&c, value, Some(key), budget)?;
                            }
                            Pending::FinishComprehension(c) => {
                                let output = std::mem::replace(
                                    &mut *c.output.borrow_mut(),
                                    Output::Array(Vec::new()),
                                );
                                result = owned!(match output {
                                    Output::Array(values) => ConstructedValue::Array(values),
                                    Output::Set(elements, element_type) => ConstructedValue::Set {
                                        elements,
                                        element_type
                                    },
                                    Output::Keyed(entries) if entries.is_empty() => {
                                        *c.output.borrow_mut() = Output::Keyed(entries);
                                        let sources = c.empty.ok_or_else(|| {
                                            error(&location, "unestablished empty keyed axes")
                                        })?;
                                        let axes =
                                            admit_container(sources.len(), budget, &location)?;
                                        if let Some(source) = sources.first() {
                                            push!(Pending::EmptyAxes(Rc::clone(&c), 0, axes));
                                            push!(Pending::Eval(source, c.depth + 1, false));
                                            return Ok(());
                                        }
                                        finish_keyed(Some(axes), Vec::new(), budget, &location)?
                                    }
                                    Output::Keyed(entries) =>
                                        finish_keyed(None, entries, budget, &location)?,
                                });
                            }
                            Pending::EmptyAxes(c, index, mut axes) => {
                                let sources = c.empty.expect("empty sources");
                                axes.push(source_axis(
                                    result
                                        .take()
                                        .expect("axis source")
                                        .materialize(budget, &location)?,
                                    budget,
                                    &Location::Query(sources[index].range.clone()),
                                )?);
                                if let Some(source) = sources.get(index + 1) {
                                    push!(Pending::EmptyAxes(Rc::clone(&c), index + 1, axes));
                                    push!(Pending::Eval(source, c.depth + 1, false));
                                } else {
                                    result = owned!(finish_keyed(
                                        Some(axes),
                                        Vec::new(),
                                        budget,
                                        &location
                                    )?);
                                }
                            }
                        }
                        Ok(())
                    },
                )?;
            }
            Ok(result.expect("entry result"))
        })();
        if outcome.is_err() {
            let calls = frames
                .iter()
                .filter_map(|frame| frame.function.map(|id| (id, frame.range.clone())))
                .rev()
                .take(16)
                .collect::<Vec<_>>();
            host.failure(&calls, frames.len().saturating_sub(1 + calls.len()));
        }
        outcome
    })
}

fn admit<'a, Q, H: Host<'a, Q>>(
    value: &mut ResultValue<'a, H, Q>,
    expected: &'a DeclaredType<H::Kind>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    match &value.data {
        Data::Native(value) => host.admit_native(value, expected, budget, location.clone())?,
        Data::Constructed(data) => {
            if !value
                .contract
                .is_some_and(|previous| std::ptr::eq(previous, expected))
            {
                let admitted = match value.contract {
                    Some(previous) => crate::runtime_types::same_declaration(
                        previous, expected, budget, location,
                    )?,
                    None => false,
                };
                if !admitted {
                    crate::admit_constructed(data.as_ref(), expected, budget, location.clone())?;
                }
            }
        }
    }
    value.contract = Some(expected);
    Ok(())
}
fn append<Q>(
    c: &Comprehension<'_, Q>,
    value: ConstructedValue,
    key: Option<Vec<ValueKey>>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let location = Location::Query(c.expression.range.clone());
    match &mut *c.output.borrow_mut() {
        Output::Array(values) => {
            allocation::<ConstructedValue>(sum(values.len(), 1, &location)?, &location)?;
            budget.charge_results(1, location)?;
            values.push(value);
        }
        Output::Set(elements, ty) => insert_set(elements, ty, value, true, budget, &location)?,
        Output::Keyed(entries) => {
            let expression = c.key.expect("key expression");
            let location = Location::Query(expression.range.clone());
            let keys = key.expect("keyed value");
            allocation::<Located<KeyedEntry>>(sum(entries.len(), 1, &location)?, &location)?;
            budget.charge(1, location.clone())?;
            budget.charge_results(1, location)?;
            entries.push(Located::new(
                KeyedEntry { key: keys, value },
                expression.range.clone(),
            ));
        }
    }
    Ok(())
}

fn operands<'a, Q>(
    expression: &'a PreparedValue<'a, Q>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<&'a PreparedValue<'a, Q>>, QueryError> {
    let count = match &expression.value {
        PreparedKind::Array(v) | PreparedKind::Tuple(v) | PreparedKind::Set(v) => v.len(),
        PreparedKind::Record(v) => v.len(),
        PreparedKind::KeyedArray { entries, .. } => entries
            .len()
            .checked_mul(2)
            .ok_or_else(|| error(location, "operand count overflow"))?,
        PreparedKind::Binary { .. }
        | PreparedKind::Lookup { .. }
        | PreparedKind::HasKey { .. }
        | PreparedKind::ArrayAccess { .. }
        | PreparedKind::Range { .. }
        | PreparedKind::ZipExact { .. } => 2,
        _ => 1,
    };
    let mut result = metadata_vec(count, budget, location)?;
    match &expression.value {
        PreparedKind::Array(v) | PreparedKind::Tuple(v) | PreparedKind::Set(v) => result.extend(v),
        PreparedKind::Record(v) => result.extend(v.iter().map(|(_, value)| value)),
        PreparedKind::KeyedArray { entries, .. } => {
            for (key, value) in entries {
                result.extend([key, value]);
            }
        }
        PreparedKind::Binary { left, right, .. } | PreparedKind::ZipExact { left, right } => {
            result.extend([left.as_ref(), right.as_ref()])
        }
        PreparedKind::Lookup { value, key } | PreparedKind::HasKey { value, key } => {
            result.extend([value.as_ref(), key.as_ref()])
        }
        PreparedKind::ArrayAccess { array, index } => {
            result.extend([array.as_ref(), index.as_ref()])
        }
        PreparedKind::Range { lower, upper } => result.extend([lower.as_ref(), upper.as_ref()]),
        PreparedKind::One(value)
        | PreparedKind::Occurs(value)
        | PreparedKind::Absent(value)
        | PreparedKind::IndexSet(value)
        | PreparedKind::Entries(value)
        | PreparedKind::Unary { operand: value, .. }
        | PreparedKind::Field { value, .. }
        | PreparedKind::TupleAccess { value, .. }
        | PreparedKind::Reduce { array: value, .. }
        | PreparedKind::GroupBy { rows: value, .. } => result.push(value),
        _ => unreachable!("special control has no ordinary operands"),
    }
    Ok(result)
}

fn apply<'a, Q, N, K>(
    expression: &'a PreparedValue<'a, Q>,
    mut values: Vec<Value<'a, N, K>>,
    budget: &mut Budget,
) -> Result<Value<'a, N, K>, QueryError> {
    let location = Location::Query(expression.range.clone());
    if let PreparedKind::Field { field, .. } = &expression.value {
        let value = values.pop().expect("field operand");
        if let Data::Constructed(data) = value.data {
            if !matches!(data.as_ref(), ConstructedValue::Declared { .. }) {
                let ConstructedValue::Record(fields) = data.as_ref() else {
                    return Err(error(&location, "field access needs a record"));
                };
                let mut selected = None;
                for (index, (label, _)) in fields.iter().enumerate() {
                    budget.charge(
                        sum(1, label.len().min(field.value.len()), &location)?,
                        location.clone(),
                    )?;
                    if label == &field.value {
                        selected = Some(index);
                        break;
                    }
                }
                let Some(index) = selected else {
                    admit_diagnostic("missing record field: ", &field.value, budget, &location)?;
                    return Err(error(
                        &location,
                        format!("missing record field: {}", field.value),
                    ));
                };
                let contract = if let Some(contract) = value.contract {
                    let mut c = contract;
                    while let DeclaredType::Optional(child) = c {
                        c = child;
                    }
                    if let DeclaredType::Record(fields) = c {
                        Some(record_declaration(fields, &field.value, budget, &location)?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                return Ok(Value {
                    data: Data::Constructed(data.child(index, budget, &location)?),
                    contract,
                });
            }
            let value = expose(data.materialize(budget, &location)?, budget, &location)?;
            return Ok(Value::constructed(RuntimeValue::Owned(select_field(
                value, field, budget, &location,
            )?)));
        }
        return Err(error(
            &location,
            "native handles require an explicit scalar projection",
        ));
    }
    if let PreparedKind::Occurs(_) | PreparedKind::Absent(_) = &expression.value {
        let present =
            matches!(&values[0].data, Data::Native(_)) || values[0].scalar(&location)?.occurs();
        return Ok(Value::constructed(RuntimeValue::Owned(scalar(
            ConstructedValue::Boolean(if matches!(expression.value, PreparedKind::Occurs(_)) {
                present
            } else {
                !present
            }),
            budget,
            &location,
        )?)));
    }
    if let PreparedKind::Reduce { operator, .. } = &expression.value {
        return Ok(Value::constructed(RuntimeValue::Owned(reduce_ref(
            *operator,
            values[0].scalar(&location)?,
            budget,
            &location,
        )?)));
    }
    let mut arguments = metadata_vec(values.len(), budget, &location)?;
    for value in values {
        arguments.push(value.materialize(budget, &location)?);
    }
    let mut arguments = arguments.into_iter();
    let mut next = || arguments.next().expect("prepared operand");
    let value = match &expression.value {
        PreparedKind::One(_) => {
            let ConstructedValue::Array(values) = expose(next(), budget, &location)? else {
                return Err(error(&location, "one needs a positional array"));
            };
            take_one(values, expression.range.clone())?
        }
        PreparedKind::Lookup { .. } | PreparedKind::HasKey { .. } => {
            let (present, value) = optional_access(next(), next(), budget, &location)?;
            if matches!(expression.value, PreparedKind::HasKey { .. }) {
                scalar(ConstructedValue::Boolean(present), budget, &location)?
            } else {
                value
            }
        }
        PreparedKind::Array(_) | PreparedKind::Tuple(_) | PreparedKind::Set(_) => {
            let mut values = admit_container(arguments.len(), budget, &location)?;
            let mut element_type = None;
            for value in arguments {
                if matches!(expression.value, PreparedKind::Set(_)) {
                    insert_set(
                        &mut values,
                        &mut element_type,
                        value,
                        false,
                        budget,
                        &location,
                    )?;
                } else {
                    values.push(value);
                }
            }
            match expression.value {
                PreparedKind::Array(_) => ConstructedValue::Array(values),
                PreparedKind::Tuple(_) => ConstructedValue::Tuple(values),
                _ => ConstructedValue::Set {
                    elements: values,
                    element_type,
                },
            }
        }
        PreparedKind::Record(fields) => {
            let mut result = admit_container(fields.len(), budget, &location)?;
            for ((label, _), value) in fields.iter().zip(arguments) {
                budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
                result.push((label.value.clone(), value));
            }
            ConstructedValue::Record(result)
        }
        PreparedKind::KeyedArray { axes, entries } => {
            let axes = axes
                .map(|axes| copy_axes(axes, budget, &location))
                .transpose()?;
            let mut values = admit_container(entries.len(), budget, &location)?;
            for (key, _) in entries {
                let key_location = Location::Query(key.range.clone());
                let keys = value_keys(next(), budget, &key_location)?;
                values.push(Located::new(
                    KeyedEntry {
                        key: keys,
                        value: next(),
                    },
                    key.range.clone(),
                ));
            }
            finish_keyed(axes, values, budget, &location)?
        }
        PreparedKind::Unary { operator, .. } => unary(*operator, next(), budget, &location)?,
        PreparedKind::Binary { operator, .. } => {
            binary(*operator, next(), next(), budget, &location)?
        }
        PreparedKind::TupleAccess { position, .. } => {
            let ConstructedValue::Tuple(values) = expose(next(), budget, &location)? else {
                return Err(error(&location, "tuple access needs a tuple"));
            };
            take_position(values, position.value, budget, &location)?
        }
        PreparedKind::ArrayAccess { .. } => {
            let array = expose(next(), budget, &location)?;
            let index = next();
            match array {
                ConstructedValue::Array(values) => {
                    let position = usize::try_from(integer(index, &location)?)
                        .map_err(|_| error(&location, "missing array position"))?;
                    take_position(values, position, budget, &location)?
                }
                ConstructedValue::KeyedArray { axes, entries } => {
                    let key = value_keys(index, budget, &location)?;
                    key_positions(&key, &axes, budget, &location)?;
                    let mut selected = None;
                    for entry in entries {
                        budget.charge(1, location.clone())?;
                        if equal_keys(&key, &entry.key, budget, &location)? {
                            selected = Some(entry.value);
                            break;
                        }
                    }
                    selected.ok_or_else(|| error(&location, "missing keyed array cell"))?
                }
                _ => return Err(error(&location, "array access needs an array")),
            }
        }
        PreparedKind::IndexSet(_) => match expose(next(), budget, &location)? {
            ConstructedValue::Array(values) => {
                let len =
                    i64::try_from(values.len()).map_err(|_| error(&location, "index overflow"))?;
                integer_range(1, len, budget, &location)?
            }
            ConstructedValue::KeyedArray { mut axes, .. } if axes.len() == 1 => {
                let axis = axes.pop().expect("rank one");
                let mut values = admit_container(axis.keys.len(), budget, &location)?;
                let mut element_type = Some(key_set_type(axis.key_type));
                for key in axis.keys {
                    budget.charge(1, location.clone())?;
                    insert_set(
                        &mut values,
                        &mut element_type,
                        key_value(key),
                        false,
                        budget,
                        &location,
                    )?;
                }
                ConstructedValue::Set {
                    element_type,
                    elements: values,
                }
            }
            ConstructedValue::KeyedArray { .. } => {
                return Err(error(&location, "index_set needs rank one"));
            }
            _ => return Err(error(&location, "index_set needs an array")),
        },
        PreparedKind::Entries(_) => entries(next(), budget, &location)?,
        PreparedKind::ZipExact { .. } => zip_exact(next(), next(), budget, &location)?,
        PreparedKind::GroupBy { field, .. } => group_by(next(), field, budget, &location)?,
        PreparedKind::Range { .. } => integer_range(
            integer(next(), &location)?,
            integer(next(), &location)?,
            budget,
            &location,
        )?,
        _ => unreachable!("control handled by continuations"),
    };
    Ok(Value::constructed(RuntimeValue::Owned(value)))
}

pub(super) struct ScalarHost<'f, 'a, Q, H: ?Sized, F> {
    pub(super) queries: &'f mut H,
    pub(super) frame: &'f mut Frame<'a>,
    pub(super) check: &'f mut F,
    pub(super) query: std::marker::PhantomData<Q>,
    pub(super) calls: &'f mut crate::functions::CallTrace,
}
impl<'a, 's, Q, H, F> Host<'a, Q> for ScalarHost<'_, 's, Q, H, F>
where
    H: ValueQueryHost<Q> + ?Sized,
    F: FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
{
    type Native = std::convert::Infallible;
    type Kind = ();
    type Error = H::Error;
    fn outer(
        &mut self,
        slot: usize,
        _: &str,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, ()>, Self::Error> {
        Ok(Value::constructed(self.frame.read(
            &ReadSlot::Outer(slot),
            self.queries,
            budget,
            range,
        )?))
    }
    fn native(
        &mut self,
        query: &'a Q,
        arguments: &'a [PreparedArgument<'a, Q>],
        locals: &mut [(&'a str, Value<'a, Self::Native, ()>)],
        _: &'a [Body<'a, Q, ()>],
        _: usize,
        _: Option<usize>,
        projected: bool,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, ()>, Self::Error> {
        if !projected {
            return Err(QueryError::query(
                range.clone(),
                "native values require a function request",
            )
            .into());
        }
        let location = Location::Query(range.clone());
        let mut values = metadata_vec(locals.len(), budget, &location)?;
        for (name, value) in locals {
            let Data::Constructed(value) = &mut value.data else {
                unreachable!()
            };
            values.push((
                *name,
                value
                    .share(budget, &location)?
                    .into_shared(budget, &location)?,
            ));
        }
        let mut frames = metadata_vec(arguments.len(), budget, &location)?;
        for argument in arguments {
            let mut names = metadata_vec(argument.names.len(), budget, &location)?;
            names.extend_from_slice(&argument.names);
            frames.push(Frame::new(names, budget, &location)?);
        }
        let mut arguments = PreparedArguments {
            arguments,
            frames,
            check_value: self.check,
        };
        let value = self.queries.evaluate_query_with_arguments(
            query,
            &values,
            &mut arguments,
            budget,
            range,
        )?;
        constructed_value_type(&value, budget, location)?;
        Ok(Value::constructed(RuntimeValue::Owned(value)))
    }
    fn admit_native(
        &mut self,
        value: &Self::Native,
        _: &DeclaredType<()>,
        _: &mut Budget,
        _: Location,
    ) -> Result<(), Self::Error> {
        match *value {}
    }
    fn check(
        &mut self,
        value: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        (self.check)(value, budget, location)
    }
    fn failure(&mut self, calls: &[(usize, std::ops::Range<usize>)], omitted: usize) {
        self.calls.append(calls, omitted);
    }
}

pub(super) fn run_scalar<'a, Q, H: ValueQueryHost<Q> + ?Sized>(
    expression: &'a PreparedValue<'a, Q>,
    queries: &mut H,
    frame: &mut Frame<'a>,
    budget: &mut Budget,
    check: &mut impl FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
) -> Result<RuntimeValue, H::Error> {
    let mut names = metadata_vec(
        frame.names.len(),
        budget,
        &Location::Query(expression.range.clone()),
    )?;
    names.extend_from_slice(&frame.names);
    let mut calls = crate::functions::CallTrace::default();
    let mut host = ScalarHost {
        queries,
        frame,
        check,
        query: std::marker::PhantomData,
        calls: &mut calls,
    };
    let value = execute(expression, &names, &[], &mut host, budget, 0, 0)?;
    let Data::Constructed(value) = value.data else {
        unreachable!()
    };
    Ok(value)
}
