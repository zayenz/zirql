use super::*;
use std::sync::Arc;

pub(crate) type PreparedValue<'a, Q> = Located<PreparedKind<'a, Q>>;

#[derive(Debug, PartialEq)]
pub(crate) enum ReadSlot {
    Outer(usize),
    Local(usize),
}

pub(crate) struct PreparedArgument<'a, Q> {
    pub source: &'a ValueExpression<Q>,
    pub expression: PreparedValue<'a, Q>,
    pub names: Vec<&'a str>,
}

pub(crate) enum PreparedKind<'a, Q> {
    Literal(&'a ConstructedValue),
    CurrentInput,
    PredicateSubject,
    Queue {
        source: usize,
        items: Box<PreparedValue<'a, Q>>,
    },
    PopFront(Box<PreparedValue<'a, Q>>),
    PushBackAll {
        queue: Box<PreparedValue<'a, Q>>,
        items: Box<PreparedValue<'a, Q>>,
    },
    IdentitySet {
        source: usize,
    },
    Insert {
        set: Box<PreparedValue<'a, Q>>,
        item: Box<PreparedValue<'a, Q>>,
    },
    Call {
        function: usize,
        arguments: Vec<PreparedValue<'a, Q>>,
    },
    Accumulate {
        items: Box<PreparedValue<'a, Q>>,
        initial: Box<PreparedValue<'a, Q>>,
        function: usize,
        step: &'a Located<String>,
        scan: bool,
    },
    NativeValue {
        query: &'a Q,
        arguments: Vec<PreparedArgument<'a, Q>>,
    },
    /// A normalized native operand; it runs only when this node is selected.
    NativeQuery {
        query: &'a Q,
        arguments: Vec<PreparedArgument<'a, Q>>,
    },
    /// Extract exactly one positional child without copying it.
    One(Box<PreparedValue<'a, Q>>),
    Name(ReadSlot),
    Occurs(Box<PreparedValue<'a, Q>>),
    Absent(Box<PreparedValue<'a, Q>>),
    Lookup {
        value: Box<PreparedValue<'a, Q>>,
        key: Box<PreparedValue<'a, Q>>,
    },
    HasKey {
        value: Box<PreparedValue<'a, Q>>,
        key: Box<PreparedValue<'a, Q>>,
    },
    Default {
        value: Box<PreparedValue<'a, Q>>,
        fallback: Box<PreparedValue<'a, Q>>,
    },
    Array(Vec<PreparedValue<'a, Q>>),
    Tuple(Vec<PreparedValue<'a, Q>>),
    Record(Vec<(&'a Located<String>, PreparedValue<'a, Q>)>),
    With {
        record: Box<PreparedValue<'a, Q>>,
        fields: Vec<(&'a Located<String>, PreparedValue<'a, Q>)>,
    },
    Dictionary(Vec<(PreparedValue<'a, Q>, PreparedValue<'a, Q>)>),
    DictionaryComprehension {
        general_keys: bool,
        clauses: Vec<Located<PreparedGenerator<'a, Q>>>,
        key: Box<PreparedValue<'a, Q>>,
        head: Box<PreparedValue<'a, Q>>,
    },
    Set(Vec<PreparedValue<'a, Q>>),
    /// Literal pairs run in written order and normalize to row-major order.
    /// Empty literals require explicit typed axes; generic sparse data is valid.
    KeyedArray {
        axes: Option<&'a [KeyAxis]>,
        declared_axis_sources: Option<Vec<PreparedValue<'a, Q>>>,
        entries: Vec<(PreparedValue<'a, Q>, PreparedValue<'a, Q>)>,
    },
    /// Both source modes resolve outside this comprehension's own binders.
    KeyedComprehension {
        clauses: Vec<Located<PreparedGenerator<'a, Q>>>,
        key: Box<PreparedValue<'a, Q>>,
        head: Box<PreparedValue<'a, Q>>,
        declared_axis_sources: Option<Vec<PreparedValue<'a, Q>>>,
        empty_axis_sources: Option<Vec<PreparedValue<'a, Q>>>,
    },
    Unary {
        operator: ValueUnaryOperator,
        operand: Box<PreparedValue<'a, Q>>,
    },
    Binary {
        operator: ValueBinaryOperator,
        left: Box<PreparedValue<'a, Q>>,
        right: Box<PreparedValue<'a, Q>>,
    },
    Field {
        value: Box<PreparedValue<'a, Q>>,
        field: &'a Located<String>,
    },
    TupleAccess {
        value: Box<PreparedValue<'a, Q>>,
        position: &'a Located<usize>,
    },
    ArrayAccess {
        array: Box<PreparedValue<'a, Q>>,
        index: Box<PreparedValue<'a, Q>>,
    },
    IndexSet(Box<PreparedValue<'a, Q>>),
    /// Expose present keyed cells as positional `key`/`value` records. Rank-one
    /// keys are scalars; higher-rank keys are tuples in dimension order.
    Entries(Box<PreparedValue<'a, Q>>),
    /// Pair keyed arrays with identical ordered axes and present full keys.
    /// Retain left axes/keys and move each pair into a two-element tuple cell.
    ZipExact {
        left: Box<PreparedValue<'a, Q>>,
        right: Box<PreparedValue<'a, Q>>,
    },
    /// Evaluate the entire array before scanning its values in stored order.
    Reduce {
        operator: ValueReducer,
        array: Box<PreparedValue<'a, Q>>,
    },
    SubjectScope {
        subject: Box<PreparedValue<'a, Q>>,
        predicate: bool,
        argument: Option<&'a Pattern>,
        body: Box<PreparedValue<'a, Q>>,
    },
    Windows {
        items: Box<PreparedValue<'a, Q>>,
        size: Box<PreparedValue<'a, Q>>,
    },
    RunsBy {
        rows: Box<PreparedValue<'a, Q>>,
        field: &'a Located<String>,
    },
    Align {
        left: Box<PreparedValue<'a, Q>>,
        right: Box<PreparedValue<'a, Q>>,
        field: &'a Located<String>,
    },
    Sort {
        array: Box<PreparedValue<'a, Q>>,
        selector: Option<&'a Located<SortSelector>>,
        descending: bool,
    },
    Unique(Box<PreparedValue<'a, Q>>),
    Tally {
        array: Box<PreparedValue<'a, Q>>,
        general_keys: bool,
        set_type: Option<SetElementType>,
    },
    Extremum {
        operator: ValueExtremum,
        array: Box<PreparedValue<'a, Q>>,
        field: Option<&'a Located<String>>,
    },
    /// Group positional records by one static field, preserving first-seen keys
    /// and all rows in input order. Keys are homogeneous integers, strings or
    /// members of one nominal type. Results use existing `key`/`items` records.
    /// Missing fields are deferred until execution; known incompatible types
    /// fail preflight. Selector/key errors use `field`'s range.
    GroupBy {
        rows: Box<PreparedValue<'a, Q>>,
        field: &'a Located<String>,
    },
    /// Preflight both operands; run value only after a true fixed Boolean.
    /// A false condition fails at this node with the literal message.
    Require {
        condition: Box<PreparedValue<'a, Q>>,
        value: Box<PreparedValue<'a, Q>>,
        message: &'a Located<String>,
    },
    Range {
        lower: Box<PreparedValue<'a, Q>>,
        upper: Box<PreparedValue<'a, Q>>,
    },
    /// Conditions are checked in order. The final else is mandatory.
    Conditional {
        branches: Vec<(PreparedValue<'a, Q>, PreparedValue<'a, Q>)>,
        otherwise: Box<PreparedValue<'a, Q>>,
    },
    Loop {
        states: Vec<(&'a Located<String>, PreparedValue<'a, Q>)>,
        body: Box<PreparedValue<'a, Q>>,
    },
    Break(Box<PreparedValue<'a, Q>>),
    Continue(Vec<PreparedValue<'a, Q>>),
    Block {
        bindings: Vec<(&'a Pattern, PreparedValue<'a, Q>)>,
        value: Box<PreparedValue<'a, Q>>,
    },
    ValueMatch {
        subject: Box<PreparedValue<'a, Q>>,
        arms: Vec<PreparedArm<'a, Q>>,
    },
    Comprehension {
        constructor: ValueConstructor,
        retain_declaration: bool,
        clauses: Vec<Located<PreparedGenerator<'a, Q>>>,
        head: Box<PreparedValue<'a, Q>>,
    },
}

pub(crate) struct PreparedArm<'a, Q> {
    pub pattern: &'a Pattern,
    pub guard: Option<PreparedValue<'a, Q>>,
    pub value: PreparedValue<'a, Q>,
}

/// Ordered generator clauses. A missing `In` pattern is the private subject slot
/// used to lower filtering and mapping; user discards are wildcard patterns.
/// Grouped `In` evaluates its source once and varies the last binder fastest.
pub(crate) enum PreparedGenerator<'a, Q> {
    In {
        case: bool,
        binders: Vec<Option<&'a Pattern>>,
        positional: bool,
        source: PreparedValue<'a, Q>,
        filter: Option<PreparedValue<'a, Q>>,
    },
    Bind {
        binder: &'a Pattern,
        value: PreparedValue<'a, Q>,
        filter: Option<PreparedValue<'a, Q>>,
    },
}

pub(crate) struct Preparation<'a> {
    pub names: Vec<&'a str>,
    pub(crate) locals: Vec<&'a str>,
    // Source addresses identify tally nodes only while their borrowed trees live.
    // Keep their empty string-set result shape after array data loses element facts.
    pub(crate) general_key_expressions: Vec<GeneralKeyExpression>,
}

impl<'a> Preparation<'a> {
    pub(crate) fn arguments<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        query: &'a Q,
        queries: &mut H,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Vec<PreparedArgument<'a, Q>>, QueryError> {
        let sources = queries.query_arguments(query, budget, range)?;
        let mut arguments = metadata_vec(sources.len(), budget, &Location::Query(range.clone()))?;
        for source in sources {
            let mut preparation = Self::new();
            preparation.general_key_expressions = std::mem::take(&mut self.general_key_expressions);
            let expression = preparation.resolve(source, queries, budget)?;
            self.general_key_expressions = preparation.general_key_expressions;
            arguments.push(PreparedArgument {
                source,
                expression,
                names: preparation.names,
            });
        }
        Ok(arguments)
    }

    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            locals: Vec::new(),
            general_key_expressions: Vec::new(),
        }
    }

    pub fn resolve<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        expression: &'a ValueExpression<Q>,
        queries: &mut H,
        budget: &mut Budget,
    ) -> Result<PreparedValue<'a, Q>, QueryError> {
        self.resolve_control(expression, queries, budget, None)
    }

    fn resolve_control<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        expression: &'a ValueExpression<Q>,
        queries: &mut H,
        budget: &mut Budget,
        transfer: Option<usize>,
    ) -> Result<PreparedValue<'a, Q>, QueryError> {
        let location = Location::Query(expression.range.clone());
        let mut resolve = |budget: &mut Budget| {
            budget.charge(1, location.clone())?;
            check_loop_terminal(&expression.value, transfer, &location)?;
            let kind = match &expression.value {
                ValueExpressionKind::Literal(value) => PreparedKind::Literal(value),
                ValueExpressionKind::CurrentInput => PreparedKind::CurrentInput,
                ValueExpressionKind::Queue { items, .. } => PreparedKind::Queue {
                    source: queries.worklist_id(expression, budget)?,
                    items: Box::new(self.resolve(items, queries, budget)?),
                },
                ValueExpressionKind::IdentitySet { .. } => PreparedKind::IdentitySet {
                    source: queries.worklist_id(expression, budget)?,
                },
                ValueExpressionKind::PopFront(q) => {
                    PreparedKind::PopFront(Box::new(self.resolve(q, queries, budget)?))
                }
                ValueExpressionKind::PushBackAll { queue, items } => PreparedKind::PushBackAll {
                    queue: Box::new(self.resolve(queue, queries, budget)?),
                    items: Box::new(self.resolve(items, queries, budget)?),
                },
                ValueExpressionKind::Insert { set, item } => PreparedKind::Insert {
                    set: Box::new(self.resolve(set, queries, budget)?),
                    item: Box::new(self.resolve(item, queries, budget)?),
                },
                ValueExpressionKind::Call {
                    function,
                    arguments,
                } => {
                    let (function, arity) = queries.function(function, budget)?;
                    if arguments.len() != arity {
                        return Err(error(&location, "function argument count mismatch"));
                    }
                    let mut values = metadata_vec(arguments.len(), budget, &location)?;
                    for value in arguments {
                        values.push(self.resolve(value, queries, budget)?);
                    }
                    PreparedKind::Call {
                        function,
                        arguments: values,
                    }
                }
                ValueExpressionKind::Fold {
                    items,
                    initial,
                    step,
                }
                | ValueExpressionKind::Scan {
                    items,
                    initial,
                    step,
                } => {
                    let (function, _) = queries.accumulator_function(step, budget)?;
                    PreparedKind::Accumulate {
                        items: Box::new(self.resolve(items, queries, budget)?),
                        initial: Box::new(self.resolve(initial, queries, budget)?),
                        function,
                        step,
                        scan: matches!(expression.value, ValueExpressionKind::Scan { .. }),
                    }
                }
                ValueExpressionKind::NativeQuery(query)
                | ValueExpressionKind::NativeValue(query) => {
                    let arguments =
                        self.arguments(query.as_ref(), queries, budget, &expression.range)?;
                    if matches!(expression.value, ValueExpressionKind::NativeValue(_)) {
                        PreparedKind::NativeValue {
                            query: query.as_ref(),
                            arguments,
                        }
                    } else {
                        PreparedKind::NativeQuery {
                            query: query.as_ref(),
                            arguments,
                        }
                    }
                }
                ValueExpressionKind::PredicateSubject => {
                    match self
                        .locals
                        .iter()
                        .rposition(|name| *name == PREDICATE_SUBJECT)
                    {
                        Some(slot) => PreparedKind::Name(ReadSlot::Local(slot)),
                        None => PreparedKind::PredicateSubject,
                    }
                }
                ValueExpressionKind::PredicateScope { subject, predicate }
                | ValueExpressionKind::SubjectScope {
                    subject,
                    argument: predicate,
                } => {
                    let subject = Box::new(self.resolve(subject, queries, budget)?);
                    let saved = self.locals.len();
                    let body = self.predicate(predicate, queries, budget, &location)?;
                    self.locals.truncate(saved);
                    PreparedKind::SubjectScope {
                        predicate: matches!(
                            expression.value,
                            ValueExpressionKind::PredicateScope { .. }
                        ),
                        subject,
                        argument: predicate.argument.as_ref(),
                        body: Box::new(body),
                    }
                }
                ValueExpressionKind::Name(name) => {
                    let work = self
                        .locals
                        .iter()
                        .chain(&self.names)
                        .try_fold(0usize, |work, local| {
                            sum(work, 1 + local.len().min(name.len()), &location)
                        })?;
                    budget.charge(work, location.clone())?;
                    let slot = if let Some(index) =
                        self.locals.iter().rposition(|local| *local == name)
                    {
                        ReadSlot::Local(index)
                    } else {
                        let index = match self.names.iter().position(|outer| *outer == name) {
                            Some(index) => index,
                            None => {
                                push_metadata(&mut self.names, name.as_str(), budget, &location)?;
                                self.names.len() - 1
                            }
                        };
                        ReadSlot::Outer(index)
                    };
                    PreparedKind::Name(slot)
                }
                ValueExpressionKind::One(value) => {
                    PreparedKind::One(Box::new(self.resolve(value, queries, budget)?))
                }
                ValueExpressionKind::Occurs(value) => {
                    PreparedKind::Occurs(Box::new(self.resolve(value, queries, budget)?))
                }
                ValueExpressionKind::Absent(value) => {
                    PreparedKind::Absent(Box::new(self.resolve(value, queries, budget)?))
                }
                ValueExpressionKind::IndexSet(value) => {
                    PreparedKind::IndexSet(Box::new(self.resolve(value, queries, budget)?))
                }
                ValueExpressionKind::Entries(value) => {
                    PreparedKind::Entries(Box::new(self.resolve(value, queries, budget)?))
                }
                ValueExpressionKind::Lookup { value, key } => PreparedKind::Lookup {
                    value: Box::new(self.resolve(value, queries, budget)?),
                    key: Box::new(self.resolve(key, queries, budget)?),
                },
                ValueExpressionKind::HasKey { value, key } => PreparedKind::HasKey {
                    value: Box::new(self.resolve(value, queries, budget)?),
                    key: Box::new(self.resolve(key, queries, budget)?),
                },
                ValueExpressionKind::Default { value, fallback } => PreparedKind::Default {
                    value: Box::new(self.resolve(value, queries, budget)?),
                    fallback: Box::new(self.resolve(fallback, queries, budget)?),
                },
                ValueExpressionKind::ZipExact { left, right } => PreparedKind::ZipExact {
                    left: Box::new(self.resolve(left, queries, budget)?),
                    right: Box::new(self.resolve(right, queries, budget)?),
                },
                ValueExpressionKind::Range { lower, upper } => PreparedKind::Range {
                    lower: Box::new(self.resolve(lower, queries, budget)?),
                    upper: Box::new(self.resolve(upper, queries, budget)?),
                },
                ValueExpressionKind::Array(values)
                | ValueExpressionKind::Tuple(values)
                | ValueExpressionKind::Set(values) => {
                    let mut children = metadata_vec(values.len(), budget, &location)?;
                    for child in values {
                        children.push(self.resolve(child, queries, budget)?);
                    }
                    match &expression.value {
                        ValueExpressionKind::Array(_) => PreparedKind::Array(children),
                        ValueExpressionKind::Tuple(_) => PreparedKind::Tuple(children),
                        _ => PreparedKind::Set(children),
                    }
                }
                ValueExpressionKind::Record(fields) => {
                    let mut children = metadata_vec(fields.len(), budget, &location)?;
                    for (label, child) in fields {
                        children.push((label, self.resolve(child, queries, budget)?));
                    }
                    PreparedKind::Record(children)
                }
                ValueExpressionKind::With { record, fields } => {
                    let record = Box::new(self.resolve(record, queries, budget)?);
                    let mut children = metadata_vec(fields.len(), budget, &location)?;
                    for (label, child) in fields {
                        children.push((label, self.resolve(child, queries, budget)?));
                    }
                    PreparedKind::With {
                        record,
                        fields: children,
                    }
                }
                ValueExpressionKind::Dictionary(entries) => {
                    let mut children = metadata_vec(entries.len(), budget, &location)?;
                    for (key, value) in entries {
                        children.push((
                            self.resolve(key, queries, budget)?,
                            self.resolve(value, queries, budget)?,
                        ));
                    }
                    PreparedKind::Dictionary(children)
                }
                ValueExpressionKind::DictionaryComprehension { clauses, key, head } => {
                    let saved = self.locals.len();
                    let clauses = self.clauses(clauses, queries, budget, &location)?;
                    let key = Box::new(self.resolve(key, queries, budget)?);
                    let head = Box::new(self.resolve(head, queries, budget)?);
                    self.locals.truncate(saved);
                    budget.charge(self.general_key_expressions.len(), location.clone())?;
                    PreparedKind::DictionaryComprehension {
                        general_keys: self
                            .general_key_expressions
                            .iter()
                            .any(|known| known.expression == expression as *const _ as usize),
                        clauses,
                        key,
                        head,
                    }
                }
                ValueExpressionKind::KeyedArray {
                    axes,
                    declared_axis_sources,
                    entries,
                } => {
                    let sources = self.axis_sources(
                        declared_axis_sources.as_deref(),
                        queries,
                        budget,
                        &location,
                    )?;
                    let mut children = metadata_vec(entries.len(), budget, &location)?;
                    for (key, value) in entries {
                        children.push((
                            self.resolve(key, queries, budget)?,
                            self.resolve(value, queries, budget)?,
                        ));
                    }
                    PreparedKind::KeyedArray {
                        axes: axes.as_deref(),
                        declared_axis_sources: sources,
                        entries: children,
                    }
                }
                ValueExpressionKind::Unary { operator, operand } => PreparedKind::Unary {
                    operator: *operator,
                    operand: Box::new(self.resolve(operand, queries, budget)?),
                },
                ValueExpressionKind::Binary {
                    operator,
                    left,
                    right,
                } => PreparedKind::Binary {
                    operator: *operator,
                    left: Box::new(self.resolve(left, queries, budget)?),
                    right: Box::new(self.resolve(right, queries, budget)?),
                },
                ValueExpressionKind::Field { value, field } => PreparedKind::Field {
                    value: Box::new(self.resolve(value, queries, budget)?),
                    field,
                },
                ValueExpressionKind::TupleAccess { value, position } => PreparedKind::TupleAccess {
                    value: Box::new(self.resolve(value, queries, budget)?),
                    position,
                },
                ValueExpressionKind::ArrayAccess { array, index } => PreparedKind::ArrayAccess {
                    array: Box::new(self.resolve(array, queries, budget)?),
                    index: Box::new(self.resolve(index, queries, budget)?),
                },
                ValueExpressionKind::Reduce { operator, array } => PreparedKind::Reduce {
                    operator: *operator,
                    array: Box::new(self.resolve(array, queries, budget)?),
                },
                ValueExpressionKind::Filter { array, predicate } => {
                    let source = self.resolve(array, queries, budget)?;
                    let saved = self.locals.len();
                    push_metadata(&mut self.locals, PREDICATE_SUBJECT, budget, &location)?;
                    let subject = Located::new(
                        PreparedKind::Name(ReadSlot::Local(saved)),
                        array.range.clone(),
                    );
                    let body = self.predicate(predicate, queries, budget, &location)?;
                    let filter = Located::new(
                        PreparedKind::SubjectScope {
                            predicate: true,
                            subject: Box::new(subject),
                            argument: predicate.argument.as_ref(),
                            body: Box::new(body),
                        },
                        predicate.body.range.clone(),
                    );
                    self.locals.truncate(saved);
                    let mut binders = metadata_vec(1, budget, &location)?;
                    binders.push(None);
                    let mut clauses = metadata_vec(1, budget, &location)?;
                    clauses.push(Located::new(
                        PreparedGenerator::In {
                            case: false,
                            binders,
                            positional: false,
                            source,
                            filter: Some(filter),
                        },
                        expression.range.clone(),
                    ));
                    PreparedKind::Comprehension {
                        constructor: ValueConstructor::Array,
                        retain_declaration: true,
                        clauses,
                        head: Box::new(Located::new(
                            PreparedKind::Name(ReadSlot::Local(saved)),
                            array.range.clone(),
                        )),
                    }
                }
                ValueExpressionKind::Map { array, argument }
                | ValueExpressionKind::FlatMap { array, argument } => self.mapping(
                    array,
                    argument,
                    matches!(expression.value, ValueExpressionKind::FlatMap { .. }),
                    queries,
                    budget,
                    &location,
                )?,
                ValueExpressionKind::Windows { items, size } => PreparedKind::Windows {
                    items: Box::new(self.resolve(items, queries, budget)?),
                    size: Box::new(self.resolve(size, queries, budget)?),
                },
                ValueExpressionKind::RunsBy { rows, field } => PreparedKind::RunsBy {
                    rows: Box::new(self.resolve(rows, queries, budget)?),
                    field,
                },
                ValueExpressionKind::Align { left, right, field } => PreparedKind::Align {
                    left: Box::new(self.resolve(left, queries, budget)?),
                    right: Box::new(self.resolve(right, queries, budget)?),
                    field,
                },
                ValueExpressionKind::Sort {
                    array,
                    selector,
                    descending,
                } => PreparedKind::Sort {
                    array: Box::new(self.resolve(array, queries, budget)?),
                    selector: selector.as_ref(),
                    descending: *descending,
                },
                ValueExpressionKind::Unique(array) => {
                    PreparedKind::Unique(Box::new(self.resolve(array, queries, budget)?))
                }
                ValueExpressionKind::Tally(array) => {
                    budget.charge(self.general_key_expressions.len(), location.clone())?;
                    let hint = self
                        .general_key_expressions
                        .iter()
                        .find(|known| known.expression == expression as *const _ as usize)
                        .copied();
                    PreparedKind::Tally {
                        general_keys: hint.is_some(),
                        set_type: hint.and_then(|hint| hint.set_type),
                        array: Box::new(self.resolve(array, queries, budget)?),
                    }
                }
                ValueExpressionKind::Extremum {
                    operator,
                    array,
                    field,
                } => PreparedKind::Extremum {
                    operator: *operator,
                    array: Box::new(self.resolve(array, queries, budget)?),
                    field: field.as_ref(),
                },
                ValueExpressionKind::GroupBy { rows, field } => PreparedKind::GroupBy {
                    rows: Box::new(self.resolve(rows, queries, budget)?),
                    field,
                },
                ValueExpressionKind::Require {
                    condition,
                    value,
                    message,
                } => PreparedKind::Require {
                    condition: Box::new(self.resolve(condition, queries, budget)?),
                    value: Box::new(self.resolve(value, queries, budget)?),
                    message,
                },
                ValueExpressionKind::Conditional {
                    branches,
                    otherwise,
                } => {
                    let mut children = metadata_vec(branches.len(), budget, &location)?;
                    for (condition, value) in branches {
                        children.push((
                            self.resolve(condition, queries, budget)?,
                            self.resolve_control(value, queries, budget, transfer)?,
                        ));
                    }
                    PreparedKind::Conditional {
                        branches: children,
                        otherwise: Box::new(
                            self.resolve_control(otherwise, queries, budget, transfer)?,
                        ),
                    }
                }
                ValueExpressionKind::Loop { states, body } => {
                    let mut resolved = metadata_vec(states.len(), budget, &location)?;
                    for (name, value) in states {
                        resolved.push((name, self.resolve(value, queries, budget)?));
                    }
                    let saved = self.locals.len();
                    for (name, _) in states {
                        push_metadata(&mut self.locals, name.value.as_str(), budget, &location)?;
                    }
                    let body = Box::new(self.resolve_control(
                        body,
                        queries,
                        budget,
                        Some(states.len()),
                    )?);
                    self.locals.truncate(saved);
                    PreparedKind::Loop {
                        states: resolved,
                        body,
                    }
                }
                ValueExpressionKind::Break(value) => {
                    PreparedKind::Break(Box::new(self.resolve(value, queries, budget)?))
                }
                ValueExpressionKind::Continue(values) => {
                    let mut resolved = metadata_vec(values.len(), budget, &location)?;
                    for value in values {
                        resolved.push(self.resolve(value, queries, budget)?);
                    }
                    PreparedKind::Continue(resolved)
                }
                ValueExpressionKind::Block { bindings, value } => {
                    let saved = self.locals.len();
                    let mut resolved = metadata_vec(bindings.len(), budget, &location)?;
                    for (pattern, value) in bindings {
                        let value = self.resolve(value, queries, budget)?;
                        pattern::names(pattern, &mut self.locals, budget)?;
                        resolved.push((pattern, value));
                    }
                    let value = Box::new(self.resolve_control(value, queries, budget, transfer)?);
                    self.locals.truncate(saved);
                    PreparedKind::Block {
                        bindings: resolved,
                        value,
                    }
                }
                ValueExpressionKind::ValueMatch { subject, arms } => {
                    let subject = Box::new(self.resolve(subject, queries, budget)?);
                    let mut resolved = metadata_vec(arms.len(), budget, &location)?;
                    for arm in arms {
                        let saved = self.locals.len();
                        pattern::names(&arm.pattern, &mut self.locals, budget)?;
                        let guard = arm
                            .guard
                            .as_ref()
                            .map(|v| self.resolve(v, queries, budget))
                            .transpose()?;
                        let value = self.resolve_control(&arm.value, queries, budget, transfer)?;
                        self.locals.truncate(saved);
                        resolved.push(PreparedArm {
                            pattern: &arm.pattern,
                            guard,
                            value,
                        });
                    }
                    PreparedKind::ValueMatch {
                        subject,
                        arms: resolved,
                    }
                }
                ValueExpressionKind::Comprehension {
                    constructor,
                    clauses,
                    head,
                } => {
                    let saved = self.locals.len();
                    let clauses = self.clauses(clauses, queries, budget, &location)?;
                    let head = Box::new(self.resolve(head, queries, budget)?);
                    self.locals.truncate(saved);
                    PreparedKind::Comprehension {
                        constructor: *constructor,
                        retain_declaration: false,
                        clauses,
                        head,
                    }
                }
                ValueExpressionKind::KeyedComprehension {
                    clauses,
                    key,
                    head,
                    declared_axis_sources,
                    empty_axis_sources,
                } => {
                    let declared = self.axis_sources(
                        declared_axis_sources.as_deref(),
                        queries,
                        budget,
                        &location,
                    )?;
                    let sources = self.axis_sources(
                        empty_axis_sources.as_deref(),
                        queries,
                        budget,
                        &location,
                    )?;
                    let saved = self.locals.len();
                    let clauses = self.clauses(clauses, queries, budget, &location)?;
                    let key = Box::new(self.resolve(key, queries, budget)?);
                    let head = Box::new(self.resolve(head, queries, budget)?);
                    self.locals.truncate(saved);
                    PreparedKind::KeyedComprehension {
                        clauses,
                        key,
                        head,
                        declared_axis_sources: declared,
                        empty_axis_sources: sources,
                    }
                }
            };
            Ok(Located::new(kind, expression.range.clone()))
        };
        if matches!(expression.value, ValueExpressionKind::NativeValue(_)) {
            resolve(budget)
        } else {
            budget.within_evaluation(1, location.clone(), resolve)
        }
    }

    fn axis_sources<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        sources: Option<&'a [ValueExpression<Q>]>,
        queries: &mut H,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Option<Vec<PreparedValue<'a, Q>>>, QueryError> {
        sources
            .map(|sources| {
                let mut children = metadata_vec(sources.len(), budget, location)?;
                for source in sources {
                    children.push(self.resolve(source, queries, budget)?);
                }
                Ok(children)
            })
            .transpose()
    }

    fn mapping<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        array: &'a ValueExpression<Q>,
        argument: &'a PredicateArgument<Q>,
        flatten: bool,
        queries: &mut H,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<PreparedKind<'a, Q>, QueryError> {
        let source = self.resolve(array, queries, budget)?;
        let saved = self.locals.len();
        push_metadata(&mut self.locals, PREDICATE_SUBJECT, budget, location)?;
        let subject = Located::new(
            PreparedKind::Name(ReadSlot::Local(saved)),
            array.range.clone(),
        );
        let body = self.predicate(argument, queries, budget, location)?;
        self.locals.truncate(saved);
        let scoped = Located::new(
            PreparedKind::SubjectScope {
                subject: Box::new(subject),
                predicate: false,
                argument: argument.argument.as_ref(),
                body: Box::new(body),
            },
            argument.body.range.clone(),
        );
        let mut clauses = metadata_vec(if flatten { 2 } else { 1 }, budget, location)?;
        let mut binders = metadata_vec(1, budget, location)?;
        binders.push(None);
        clauses.push(Located::new(
            PreparedGenerator::In {
                case: false,
                binders,
                positional: true,
                source,
                filter: None,
            },
            array.range.clone(),
        ));
        let head = if flatten {
            let mut binders = metadata_vec(1, budget, location)?;
            binders.push(None);
            clauses.push(Located::new(
                PreparedGenerator::In {
                    case: false,
                    binders,
                    positional: true,
                    source: scoped,
                    filter: None,
                },
                argument.body.range.clone(),
            ));
            Located::new(
                PreparedKind::Name(ReadSlot::Local(saved + 1)),
                argument.body.range.clone(),
            )
        } else {
            scoped
        };
        Ok(PreparedKind::Comprehension {
            constructor: ValueConstructor::Array,
            retain_declaration: false,
            clauses,
            head: Box::new(head),
        })
    }

    fn predicate<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        predicate: &'a PredicateArgument<Q>,
        queries: &mut H,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<PreparedValue<'a, Q>, QueryError> {
        push_metadata(&mut self.locals, PREDICATE_SUBJECT, budget, location)?;
        if let Some(pattern) = &predicate.argument {
            pattern::names(pattern, &mut self.locals, budget)?;
        }
        self.resolve(&predicate.body, queries, budget)
    }

    fn clauses<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        clauses: &'a [Located<ValueGenerator<Q>>],
        queries: &mut H,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<Located<PreparedGenerator<'a, Q>>>, QueryError> {
        let mut result = metadata_vec(clauses.len(), budget, location)?;
        for clause in clauses {
            let value = match &clause.value {
                ValueGenerator::In {
                    case,
                    binders,
                    source,
                    filter,
                } => {
                    let source = self.resolve(source, queries, budget)?;
                    let mut names = metadata_vec(binders.len(), budget, location)?;
                    for binder in binders {
                        names.push(Some(binder));
                        pattern::names(binder, &mut self.locals, budget)?;
                    }
                    let filter = filter
                        .as_ref()
                        .map(|value| self.resolve(value, queries, budget))
                        .transpose()?;
                    PreparedGenerator::In {
                        case: *case,
                        binders: names,
                        positional: false,
                        source,
                        filter,
                    }
                }
                ValueGenerator::Bind {
                    binder,
                    value,
                    filter,
                } => {
                    let value = self.resolve(value, queries, budget)?;
                    pattern::names(binder, &mut self.locals, budget)?;
                    let filter = filter
                        .as_ref()
                        .map(|value| self.resolve(value, queries, budget))
                        .transpose()?;
                    PreparedGenerator::Bind {
                        binder,
                        value,
                        filter,
                    }
                }
            };
            result.push(Located::new(value, clause.range.clone()));
        }
        Ok(result)
    }
}

/// An immutable selected value; paths borrow children through their retained owner.
/// Only current slots and consumers retain owners, never a history of evaluated nodes.
#[derive(Debug)]
pub(crate) enum RuntimeValue {
    Owned(ConstructedValue),
    Shared {
        owner: Arc<ConstructedValue>,
        path: Vec<usize>,
        declaration: Option<Arc<DeclaredType<()>>>,
    },
}

impl RuntimeValue {
    pub fn shared(owner: Arc<ConstructedValue>) -> Self {
        Self::Shared {
            owner,
            path: Vec::new(),
            declaration: None,
        }
    }
    pub fn as_ref(&self) -> &ConstructedValue {
        match self {
            Self::Owned(value) => value,
            Self::Shared { owner, path, .. } => {
                let mut value = owner.as_ref();
                for index in path {
                    value = match value {
                        ConstructedValue::Declared { value, .. } => value,
                        ConstructedValue::Record(fields) => &fields[*index].1,
                        ConstructedValue::Dictionary(entries) => &entries[*index].1,
                        ConstructedValue::Array(values) | ConstructedValue::Tuple(values) => {
                            &values[*index]
                        }
                        ConstructedValue::Set { elements, .. } => &elements[*index],
                        ConstructedValue::KeyedArray { entries, .. } => &entries[*index].value,
                        _ => unreachable!("shared child path"),
                    };
                }
                value
            }
        }
    }
    pub fn declaration(&self) -> Option<&DeclaredType<()>> {
        super::declaration(self.as_ref()).or(match self {
            Self::Shared { declaration, .. } => declaration.as_deref(),
            Self::Owned(_) => None,
        })
    }
    pub fn retain_declaration(
        &mut self,
        contract: &Arc<DeclaredType<()>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        budget.charge(1, location.clone())?;
        if let Self::Owned(_) = self {
            let Self::Owned(value) = std::mem::replace(self, Self::Owned(ConstructedValue::Absent))
            else {
                unreachable!()
            };
            *self = Self::shared(Arc::new(value));
        }
        let Self::Shared { declaration, .. } = self else {
            unreachable!()
        };
        *declaration = Some(Arc::clone(contract));
        Ok(())
    }
    pub fn materialize(
        self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<ConstructedValue, QueryError> {
        match self {
            Self::Owned(value) => Ok(value),
            shared => {
                let value = copy_constructed_value(shared.as_ref(), budget, location.clone())?;
                if let Self::Shared {
                    declaration: Some(declaration),
                    ..
                } = shared
                {
                    annotate(
                        value,
                        copy_declaration(&declaration, budget, location)?,
                        budget,
                        location,
                    )
                } else {
                    Ok(value)
                }
            }
        }
    }
    pub fn into_shared(
        self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Arc<ConstructedValue>, QueryError> {
        if let Self::Shared {
            owner,
            path,
            declaration,
        } = self
        {
            if path.is_empty() && declaration.is_none() {
                return Ok(owner);
            }
            let view = Self::Shared {
                owner,
                path,
                declaration,
            };
            let value = view.materialize(budget, location)?;
            budget.charge(1, location.clone())?;
            Ok(Arc::new(value))
        } else {
            budget.charge(1, location.clone())?;
            Ok(Arc::new(self.materialize(budget, location)?))
        }
    }
    pub fn share(&mut self, budget: &mut Budget, location: &Location) -> Result<Self, QueryError> {
        budget.charge(1, location.clone())?;
        if let Self::Owned(_) = self {
            let Self::Owned(value) = std::mem::replace(self, Self::Owned(ConstructedValue::Absent))
            else {
                unreachable!()
            };
            *self = Self::shared(Arc::new(value));
        }
        let Self::Shared {
            owner,
            path,
            declaration,
        } = self
        else {
            unreachable!()
        };
        budget.charge(path.len() + 1, location.clone())?;
        Ok(Self::Shared {
            owner: Arc::clone(owner),
            path: path.clone(),
            declaration: declaration.as_ref().map(Arc::clone),
        })
    }
    pub fn into_data(
        mut self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        while matches!(self.as_ref(), ConstructedValue::Declared { .. }) {
            budget.charge(1, location.clone())?;
            match &mut self {
                Self::Owned(value) => {
                    let ConstructedValue::Declared { value: inner, .. } =
                        std::mem::replace(value, ConstructedValue::Absent)
                    else {
                        unreachable!()
                    };
                    *value = *inner;
                }
                Self::Shared { path, .. } => push_metadata(path, 0, budget, location)?,
            }
        }
        Ok(self)
    }
    pub fn into_present(
        self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        let inherited = match &self {
            Self::Shared { declaration, .. } => declaration.as_deref(),
            _ => None,
        };
        let contract = super::declaration(self.as_ref())
            .or(inherited)
            .map(underlying)
            .map(|d| copy_declaration(d, budget, location))
            .transpose()?;
        let value = self.into_data(budget, location)?;
        Ok(match (value, contract) {
            (Self::Owned(value), Some(contract)) => {
                Self::Owned(annotate(value, contract, budget, location)?)
            }
            (Self::Shared { owner, path, .. }, contract) => Self::Shared {
                owner,
                path,
                declaration: contract.map(Arc::new),
            },
            (value, None) => value,
        })
    }
    pub fn pattern_child(
        &mut self,
        index: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        let inherited = match &*self {
            Self::Shared { declaration, .. } => declaration.as_deref(),
            _ => None,
        };
        let contract = super::declaration(self.as_ref())
            .or(inherited)
            .map(underlying);
        let child = match (contract, self.as_ref().data()) {
            (Some(DeclaredType::Tuple(children)), ConstructedValue::Tuple(_)) => {
                children.get(index)
            }
            (
                Some(DeclaredType::Array(child) | DeclaredType::Sequence(child)),
                ConstructedValue::Array(_),
            ) => Some(child.as_ref()),
            (Some(DeclaredType::Record(fields)), ConstructedValue::Record(values)) => Some(
                record_declaration(fields, &values[index].0, budget, location)?,
            ),
            _ => None,
        }
        .map(|child| copy_declaration(child, budget, location))
        .transpose()?;
        let mut value = self.share(budget, location)?.into_data(budget, location)?;
        // This path already selected the contract above. Ordinary child access
        // below selects inherited contracts itself.
        if let Self::Shared { declaration, .. } = &mut value {
            *declaration = None;
        }
        let mut value = value.child(index, budget, location)?;
        if let Self::Shared { declaration, .. } = &mut value {
            *declaration = child.map(Arc::new);
        }
        Ok(value)
    }
    pub fn child(
        mut self,
        index: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        let inherited = match &self {
            Self::Shared { declaration, .. } => declaration.as_deref().map(underlying),
            _ => None,
        };
        let contract = match (inherited, self.as_ref().data()) {
            (Some(DeclaredType::Dictionary(_, child)), _) => Some(child.as_ref()),
            (Some(DeclaredType::Tuple(children)), _) => children.get(index),
            (Some(DeclaredType::Record(fields)), ConstructedValue::Record(values)) => Some(
                record_declaration(fields, &values[index].0, budget, location)?,
            ),
            (
                Some(
                    DeclaredType::Array(child)
                    | DeclaredType::Sequence(child)
                    | DeclaredType::Keyed(child),
                ),
                _,
            ) => Some(child.as_ref()),
            _ => None,
        }
        .map(|child| copy_declaration(child, budget, location))
        .transpose()?;
        match &mut self {
            Self::Owned(value) => {
                let value = std::mem::replace(value, ConstructedValue::Absent);
                let selected = match value {
                    ConstructedValue::Dictionary(entries) => {
                        entries
                            .into_iter()
                            .nth(index)
                            .expect("selected dictionary value")
                            .1
                    }
                    ConstructedValue::Record(fields) => {
                        fields.into_iter().nth(index).expect("selected field").1
                    }
                    ConstructedValue::Array(values) | ConstructedValue::Tuple(values) => {
                        values.into_iter().nth(index).expect("selected position")
                    }
                    ConstructedValue::Set { elements, .. } => {
                        elements.into_iter().nth(index).expect("selected position")
                    }
                    ConstructedValue::KeyedArray { entries, .. } => {
                        entries.into_iter().nth(index).expect("selected cell").value
                    }
                    _ => unreachable!("selected child"),
                };
                Ok(Self::Owned(selected))
            }
            Self::Shared {
                path, declaration, ..
            } => {
                push_metadata(path, index, budget, location)?;
                *declaration = contract.map(Arc::new);
                Ok(self)
            }
        }
    }
}

pub(crate) struct Frame<'a> {
    pub(crate) names: Vec<&'a str>,
    pub(crate) outer: Vec<Option<RuntimeValue>>,
    current_input: Option<RuntimeValue>,
    pub(crate) locals: Vec<(&'a str, RuntimeValue)>,
}
impl<'a> Frame<'a> {
    pub fn new(
        names: Vec<&'a str>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        let mut outer = metadata_vec(names.len(), budget, location)?;
        outer.resize_with(names.len(), || None);
        Ok(Self {
            names,
            outer,
            current_input: None,
            locals: Vec::new(),
        })
    }
    pub(crate) fn current_input<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        queries: &mut H,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<RuntimeValue, H::Error> {
        let location = Location::Query(range.clone());
        if self.current_input.is_none() {
            let value = queries.read_current_input(range, budget)?;
            if let ConstructedValue::Array(rows) = value.data() {
                budget.check_items(rows.len(), location.clone())?;
            }
            constructed_value_type(&value, budget, location.clone())?;
            self.current_input = Some(RuntimeValue::Owned(value));
        }
        Ok(self
            .current_input
            .as_mut()
            .expect("retained current input")
            .share(budget, &location)?)
    }
    pub fn named_slot(
        &self,
        name: &str,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Option<usize>, QueryError> {
        for (index, outer) in self.names.iter().enumerate() {
            budget.charge(
                1 + outer.len().min(name.len()),
                Location::Query(range.clone()),
            )?;
            if *outer == name {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }
    pub fn reset(&mut self, slot: Option<usize>) {
        if let Some(index) = slot {
            self.outer[index] = None;
        }
    }
    pub(crate) fn read<Q, H: ValueQueryHost<Q> + ?Sized>(
        &mut self,
        slot: &ReadSlot,
        queries: &mut H,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<RuntimeValue, H::Error> {
        let location = Location::Query(range.clone());
        budget.charge(1, location.clone())?;
        match *slot {
            ReadSlot::Local(index) => Ok(self.locals[index].1.share(budget, &location)?),
            ReadSlot::Outer(index) => {
                if self.outer[index].is_none() {
                    let value =
                        match queries.read_shared_binding(self.names[index], range, budget)? {
                            Some(value) => RuntimeValue::shared(value),
                            None => RuntimeValue::Owned(queries.read_binding(
                                self.names[index],
                                range,
                                budget,
                            )?),
                        };
                    constructed_value_type(value.as_ref(), budget, location.clone())?;
                    self.outer[index] = Some(value);
                }
                Ok(self.outer[index]
                    .as_mut()
                    .expect("selected outer slot")
                    .share(budget, &location)?)
            }
        }
    }
}
