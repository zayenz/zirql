//! Structural operations over shared syntax. Native payloads remain opaque.
use super::{
    PredicateArgument, ValueExpression, ValueExpressionKind as K, ValueGenerator, ValueMatchArm,
};
use crate::{Budget, Located, Location, QueryError};

/// Inspect direct children in written order, without evaluating them.
/// Dictionary keys precede values; declared axes precede generators, whose
/// sources precede filters, then comprehension key/head and zero-cell fallbacks.
/// Native payloads, patterns and declarations are not expression children.
pub fn for_each_value_child<'a, Q>(
    value: &'a ValueExpression<Q>,
    mut visit: impl FnMut(&'a ValueExpression<Q>),
) {
    match &value.value {
        K::Literal(_)
        | K::CurrentInput
        | K::Name(_)
        | K::PredicateSubject
        | K::NativeQuery(_)
        | K::NativeValue(_)
        | K::IdentitySet { .. } => {}
        K::Array(values)
        | K::Tuple(values)
        | K::Set(values)
        | K::Continue(values)
        | K::Call {
            arguments: values, ..
        } => {
            for value in values {
                visit(value);
            }
        }
        K::Record(fields) => {
            for (_, value) in fields {
                visit(value);
            }
        }
        K::With { record, fields } => {
            visit(record);
            for (_, value) in fields {
                visit(value);
            }
        }
        K::Dictionary(entries) => {
            for (key, value) in entries {
                visit(key);
                visit(value);
            }
        }
        K::KeyedArray {
            declared_axis_sources,
            entries,
            ..
        } => {
            for source in declared_axis_sources.iter().flatten() {
                visit(source);
            }
            for (key, value) in entries {
                visit(key);
                visit(value);
            }
        }
        K::Comprehension { clauses, head, .. } => {
            generators(clauses, &mut visit);
            visit(head);
        }
        K::DictionaryComprehension { clauses, key, head } => {
            generators(clauses, &mut visit);
            visit(key);
            visit(head);
        }
        K::KeyedComprehension {
            clauses,
            key,
            head,
            declared_axis_sources,
            empty_axis_sources,
        } => {
            for source in declared_axis_sources.iter().flatten() {
                visit(source);
            }
            generators(clauses, &mut visit);
            visit(key);
            visit(head);
            for source in empty_axis_sources.iter().flatten() {
                visit(source);
            }
        }
        K::SubjectScope { subject, argument } => {
            visit(subject);
            visit(&argument.body);
        }
        K::PredicateScope { subject, predicate } => {
            visit(subject);
            visit(&predicate.body);
        }
        K::Filter { array, predicate } => {
            visit(array);
            visit(&predicate.body);
        }
        K::Map { array, argument } | K::FlatMap { array, argument } => {
            visit(array);
            visit(&argument.body);
        }
        K::Head { collection, count } | K::Tail { collection, count } => {
            visit(collection);
            visit(count);
        }
        K::Check {
            collection,
            operand,
            message,
        } => {
            visit(collection);
            if let Some(operand) = operand {
                visit(operand);
            }
            if let Some(message) = message {
                visit(message);
            }
        }
        K::Queue { items: value, .. }
        | K::Reverse(value)
        | K::PopFront(value)
        | K::Break(value)
        | K::One(value)
        | K::Occurs(value)
        | K::Absent(value)
        | K::IndexSet(value)
        | K::Entries(value)
        | K::Unique(value)
        | K::Tally(value)
        | K::Unary { operand: value, .. }
        | K::Field { value, .. }
        | K::TupleAccess { value, .. }
        | K::Reduce { array: value, .. }
        | K::Sort { array: value, .. }
        | K::Extremum { array: value, .. }
        | K::GroupBy { rows: value, .. }
        | K::RunsBy { rows: value, .. } => visit(value),
        K::PushBackAll {
            queue: left,
            items: right,
        }
        | K::Insert {
            set: left,
            item: right,
        }
        | K::Lookup {
            value: left,
            key: right,
        }
        | K::HasKey {
            value: left,
            key: right,
        }
        | K::Default {
            value: left,
            fallback: right,
        }
        | K::Binary { left, right, .. }
        | K::ZipExact { left, right }
        | K::Align { left, right, .. }
        | K::Windows {
            items: left,
            size: right,
        }
        | K::Fold {
            items: left,
            initial: right,
            ..
        }
        | K::Scan {
            items: left,
            initial: right,
            ..
        }
        | K::ArrayAccess {
            array: left,
            index: right,
        }
        | K::Range {
            lower: left,
            upper: right,
        }
        | K::Require {
            condition: left,
            value: right,
            ..
        } => {
            visit(left);
            visit(right);
        }
        K::Conditional {
            branches,
            otherwise,
        } => {
            for (condition, value) in branches {
                visit(condition);
                visit(value);
            }
            visit(otherwise);
        }
        K::Loop { states, body } => {
            for (_, value) in states {
                visit(value);
            }
            visit(body);
        }
        K::Block { bindings, value } => {
            for (_, value) in bindings {
                visit(value);
            }
            visit(value);
        }
        K::ValueMatch { subject, arms } => {
            visit(subject);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visit(guard);
                }
                visit(&arm.value);
            }
        }
    }
}
fn generators<'a, Q>(
    clauses: &'a [Located<ValueGenerator<Q>>],
    visit: &mut impl FnMut(&'a ValueExpression<Q>),
) {
    for clause in clauses {
        match &clause.value {
            ValueGenerator::In { source, filter, .. }
            | ValueGenerator::Bind {
                value: source,
                filter,
                ..
            } => {
                visit(source);
                if let Some(filter) = filter {
                    visit(filter);
                }
            }
        }
    }
}

/// Mutate direct children in the same order as [`for_each_value_child`].
/// This representation-only operation does not allocate or consume a budget.
pub fn for_each_value_child_mut<'a, Q>(
    value: &'a mut ValueExpression<Q>,
    mut visit: impl FnMut(&'a mut ValueExpression<Q>),
) {
    match &mut value.value {
        K::Literal(_)
        | K::CurrentInput
        | K::Name(_)
        | K::PredicateSubject
        | K::NativeQuery(_)
        | K::NativeValue(_)
        | K::IdentitySet { .. } => {}
        K::Array(values)
        | K::Tuple(values)
        | K::Set(values)
        | K::Continue(values)
        | K::Call {
            arguments: values, ..
        } => {
            for value in values {
                visit(value);
            }
        }
        K::Record(fields) => {
            for (_, value) in fields {
                visit(value);
            }
        }
        K::With { record, fields } => {
            visit(record);
            for (_, value) in fields {
                visit(value);
            }
        }
        K::Dictionary(entries) => {
            for (key, value) in entries {
                visit(key);
                visit(value);
            }
        }
        K::KeyedArray {
            declared_axis_sources,
            entries,
            ..
        } => {
            for source in declared_axis_sources.iter_mut().flatten() {
                visit(source);
            }
            for (key, value) in entries {
                visit(key);
                visit(value);
            }
        }
        K::Comprehension { clauses, head, .. } => {
            generators_mut(clauses, &mut visit);
            visit(head);
        }
        K::DictionaryComprehension { clauses, key, head } => {
            generators_mut(clauses, &mut visit);
            visit(key);
            visit(head);
        }
        K::KeyedComprehension {
            clauses,
            key,
            head,
            declared_axis_sources,
            empty_axis_sources,
        } => {
            for source in declared_axis_sources.iter_mut().flatten() {
                visit(source);
            }
            generators_mut(clauses, &mut visit);
            visit(key);
            visit(head);
            for source in empty_axis_sources.iter_mut().flatten() {
                visit(source);
            }
        }
        K::SubjectScope { subject, argument } => {
            visit(subject);
            visit(&mut argument.body);
        }
        K::PredicateScope { subject, predicate } => {
            visit(subject);
            visit(&mut predicate.body);
        }
        K::Filter { array, predicate } => {
            visit(array);
            visit(&mut predicate.body);
        }
        K::Map { array, argument } | K::FlatMap { array, argument } => {
            visit(array);
            visit(&mut argument.body);
        }
        K::Head { collection, count } | K::Tail { collection, count } => {
            visit(collection);
            visit(count);
        }
        K::Check {
            collection,
            operand,
            message,
        } => {
            visit(collection);
            if let Some(operand) = operand {
                visit(operand);
            }
            if let Some(message) = message {
                visit(message);
            }
        }
        K::Queue { items: value, .. }
        | K::Reverse(value)
        | K::PopFront(value)
        | K::Break(value)
        | K::One(value)
        | K::Occurs(value)
        | K::Absent(value)
        | K::IndexSet(value)
        | K::Entries(value)
        | K::Unique(value)
        | K::Tally(value)
        | K::Unary { operand: value, .. }
        | K::Field { value, .. }
        | K::TupleAccess { value, .. }
        | K::Reduce { array: value, .. }
        | K::Sort { array: value, .. }
        | K::Extremum { array: value, .. }
        | K::GroupBy { rows: value, .. }
        | K::RunsBy { rows: value, .. } => visit(value),
        K::PushBackAll {
            queue: left,
            items: right,
        }
        | K::Insert {
            set: left,
            item: right,
        }
        | K::Lookup {
            value: left,
            key: right,
        }
        | K::HasKey {
            value: left,
            key: right,
        }
        | K::Default {
            value: left,
            fallback: right,
        }
        | K::Binary { left, right, .. }
        | K::ZipExact { left, right }
        | K::Align { left, right, .. }
        | K::Windows {
            items: left,
            size: right,
        }
        | K::Fold {
            items: left,
            initial: right,
            ..
        }
        | K::Scan {
            items: left,
            initial: right,
            ..
        }
        | K::ArrayAccess {
            array: left,
            index: right,
        }
        | K::Range {
            lower: left,
            upper: right,
        }
        | K::Require {
            condition: left,
            value: right,
            ..
        } => {
            visit(left);
            visit(right);
        }
        K::Conditional {
            branches,
            otherwise,
        } => {
            for (condition, value) in branches {
                visit(condition);
                visit(value);
            }
            visit(otherwise);
        }
        K::Loop { states, body } => {
            for (_, value) in states {
                visit(value);
            }
            visit(body);
        }
        K::Block { bindings, value } => {
            for (_, value) in bindings {
                visit(value);
            }
            visit(value);
        }
        K::ValueMatch { subject, arms } => {
            visit(subject);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    visit(guard);
                }
                visit(&mut arm.value);
            }
        }
    }
}
fn generators_mut<'a, Q>(
    clauses: &'a mut [Located<ValueGenerator<Q>>],
    visit: &mut impl FnMut(&'a mut ValueExpression<Q>),
) {
    for clause in clauses {
        match &mut clause.value {
            ValueGenerator::In { source, filter, .. }
            | ValueGenerator::Bind {
                value: source,
                filter,
                ..
            } => {
                visit(source);
                if let Some(filter) = filter {
                    visit(filter);
                }
            }
        }
    }
}

/// Count direct children without allocating traversal storage.
pub fn value_child_count<Q>(value: &ValueExpression<Q>) -> usize {
    let mut count = 0usize;
    for_each_value_child(value, |_| count = count.saturating_add(1));
    count
}

/// Visit written syntax in preorder with the active request budget.
/// The callback sees depth zero at the root. It may inspect native payloads but
/// this walk never descends into them or applies evaluation/scope rules.
///
/// # Errors
/// Callback errors and work, result-storage or nesting refusals retain locations.
pub fn walk_value<'a, Q>(
    value: &'a ValueExpression<Q>,
    budget: &mut Budget,
    nesting: usize,
    mut visit: impl FnMut(&'a ValueExpression<Q>, usize, &mut Budget) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    admit_node(value, budget, nesting, 0)?;
    budget.charge_results(1, Location::Query(value.range.clone()))?;
    let mut pending = vec![(value, 0)];
    while let Some((value, depth)) = pending.pop() {
        visit(value, depth, budget)?;
        let width = value_child_count(value);
        admit_children(value, width, budget)?;
        let start = pending.len();
        for_each_value_child(value, |child| pending.push((child, depth + 1)));
        for (child, depth) in &pending[start..] {
            admit_node(child, budget, nesting, *depth)?;
        }
        pending[start..].reverse();
    }
    Ok(())
}

/// Mutate written syntax in preorder, then visit the node's resulting children.
///
/// # Errors
/// Uses the same budget and depth rules as [`walk_value`].
pub fn walk_value_mut<Q>(
    value: &mut ValueExpression<Q>,
    budget: &mut Budget,
    nesting: usize,
    mut visit: impl FnMut(&mut ValueExpression<Q>, usize, &mut Budget) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    admit_node(value, budget, nesting, 0)?;
    budget.charge_results(1, Location::Query(value.range.clone()))?;
    let mut pending = vec![(value, 0)];
    while let Some((value, depth)) = pending.pop() {
        visit(value, depth, budget)?;
        let width = value_child_count(value);
        admit_children(value, width, budget)?;
        let start = pending.len();
        for_each_value_child_mut(value, |child| pending.push((child, depth + 1)));
        for (child, depth) in &pending[start..] {
            admit_node(child, budget, nesting, *depth)?;
        }
        pending[start..].reverse();
    }
    Ok(())
}
fn admit_node<Q>(
    value: &ValueExpression<Q>,
    budget: &mut Budget,
    nesting: usize,
    depth: usize,
) -> Result<(), QueryError> {
    let location = Location::Query(value.range.clone());
    budget.check_evaluation_depth(depth + 1, 0, location.clone())?;
    if depth >= nesting.min(64) {
        return Err(QueryError::query(
            value.range.clone(),
            "expression nesting limit exceeded",
        ));
    }
    budget.charge(1, location)
}
fn admit_children<Q>(
    value: &ValueExpression<Q>,
    width: usize,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let location = Location::Query(value.range.clone());
    budget.charge(width, location.clone())?;
    budget.charge_results(width.saturating_add(1), location)
}

/// Move opaque native payloads through a mapper, preserving all syntax metadata.
/// Mapping visits payloads in written order. It never evaluates syntax and needs
/// no `Clone` bound. Explicit stacks handle flat/deep trees without recursion.
pub fn map_value_payloads<Q, R>(
    value: ValueExpression<Q>,
    mut mapper: impl FnMut(Q) -> R,
) -> ValueExpression<R> {
    enum Work<Q, R> {
        Visit(ValueExpression<Q>),
        Finish(ValueExpression<R>),
    }
    let mut pending = vec![Work::Visit(value)];
    let mut output = Vec::new();
    while let Some(work) = pending.pop() {
        match work {
            Work::Visit(value) => {
                let mut children = Vec::new();
                let shell = map_node(value, &mut mapper, &mut children);
                pending.push(Work::Finish(shell));
                pending.extend(children.into_iter().rev().map(Work::Visit));
            }
            Work::Finish(mut value) => {
                let count = value_child_count(&value);
                let start = output.len() - count;
                let mut children = output.drain(start..);
                for_each_value_child_mut(&mut value, |slot| {
                    *slot = children
                        .next()
                        .expect("each shell child has a mapped result")
                });
                drop(children);
                output.push(value);
            }
        }
    }
    output.pop().expect("the root produces one mapped value")
}
fn child<Q, R>(
    value: ValueExpression<Q>,
    children: &mut Vec<ValueExpression<Q>>,
) -> ValueExpression<R> {
    children.push(value);
    Located::new(K::CurrentInput, 0..0)
}

fn map_node<Q, R>(
    value: ValueExpression<Q>,
    lower: &mut impl FnMut(Q) -> R,
    children: &mut Vec<ValueExpression<Q>>,
) -> ValueExpression<R> {
    let range = value.range;
    let value = match value.value {
        K::CurrentInput => K::CurrentInput,
        K::Literal(value) => K::Literal(value),
        K::Name(name) => K::Name(name),
        K::NativeQuery(query) => K::NativeQuery(Box::new(lower(*query))),
        K::NativeValue(query) => K::NativeValue(Box::new(lower(*query))),
        K::Call {
            function,
            arguments,
        } => K::Call {
            function,
            arguments: arguments
                .into_iter()
                .map(|value| child(value, children))
                .collect(),
        },
        K::Array(values) => K::Array(values.into_iter().map(|v| child(v, children)).collect()),
        K::Tuple(values) => K::Tuple(values.into_iter().map(|v| child(v, children)).collect()),
        K::Set(values) => K::Set(values.into_iter().map(|v| child(v, children)).collect()),
        K::Dictionary(entries) => K::Dictionary(
            entries
                .into_iter()
                .map(|(key, value)| (child(key, children), child(value, children)))
                .collect(),
        ),
        K::DictionaryComprehension { clauses, key, head } => K::DictionaryComprehension {
            clauses: map_clauses(clauses, children),
            key: Box::new(child(*key, children)),
            head: Box::new(child(*head, children)),
        },
        K::With { record, fields } => K::With {
            record: Box::new(child(*record, children)),
            fields: fields
                .into_iter()
                .map(|(name, value)| (name, child(value, children)))
                .collect(),
        },
        K::Record(fields) => K::Record(
            fields
                .into_iter()
                .map(|(n, v)| (n, child(v, children)))
                .collect(),
        ),
        K::Block { bindings, value } => K::Block {
            bindings: bindings
                .into_iter()
                .map(|(p, v)| (p, child(v, children)))
                .collect(),
            value: Box::new(child(*value, children)),
        },
        K::ValueMatch { subject, arms } => K::ValueMatch {
            subject: Box::new(child(*subject, children)),
            arms: arms
                .into_iter()
                .map(|arm| ValueMatchArm {
                    pattern: arm.pattern,
                    guard: arm.guard.map(|v| child(v, children)),
                    value: child(arm.value, children),
                })
                .collect(),
        },
        K::Conditional {
            branches,
            otherwise,
        } => K::Conditional {
            branches: branches
                .into_iter()
                .map(|(a, b)| (child(a, children), child(b, children)))
                .collect(),
            otherwise: Box::new(child(*otherwise, children)),
        },
        K::KeyedArray {
            axes,
            declared_axis_sources,
            entries,
        } => K::KeyedArray {
            axes,
            declared_axis_sources: declared_axis_sources
                .map(|values| values.into_iter().map(|v| child(v, children)).collect()),
            entries: entries
                .into_iter()
                .map(|(a, b)| (child(a, children), child(b, children)))
                .collect(),
        },
        K::Comprehension {
            constructor,
            clauses,
            head,
        } => K::Comprehension {
            constructor,
            clauses: map_clauses(clauses, children),
            head: Box::new(child(*head, children)),
        },
        K::KeyedComprehension {
            clauses,
            key,
            head,
            declared_axis_sources,
            empty_axis_sources,
        } => K::KeyedComprehension {
            declared_axis_sources: declared_axis_sources
                .map(|values| values.into_iter().map(|v| child(v, children)).collect()),
            clauses: map_clauses(clauses, children),
            key: Box::new(child(*key, children)),
            head: Box::new(child(*head, children)),
            empty_axis_sources: empty_axis_sources
                .map(|values| values.into_iter().map(|v| child(v, children)).collect()),
        },
        K::Unique(value) => K::Unique(Box::new(child(*value, children))),
        K::Tally(value) => K::Tally(Box::new(child(*value, children))),
        K::Occurs(value) => K::Occurs(Box::new(child(*value, children))),
        K::Absent(value) => K::Absent(Box::new(child(*value, children))),
        K::One(value) => K::One(Box::new(child(*value, children))),
        K::IndexSet(value) => K::IndexSet(Box::new(child(*value, children))),
        K::Entries(value) => K::Entries(Box::new(child(*value, children))),
        K::Unary { operator, operand } => K::Unary {
            operator,
            operand: Box::new(child(*operand, children)),
        },
        K::Binary {
            operator,
            left,
            right,
        } => K::Binary {
            operator,
            left: Box::new(child(*left, children)),
            right: Box::new(child(*right, children)),
        },
        K::Field { value, field } => K::Field {
            value: Box::new(child(*value, children)),
            field,
        },
        K::TupleAccess { value, position } => K::TupleAccess {
            value: Box::new(child(*value, children)),
            position,
        },
        K::ArrayAccess { array, index } => K::ArrayAccess {
            array: Box::new(child(*array, children)),
            index: Box::new(child(*index, children)),
        },
        K::Lookup { value, key } => K::Lookup {
            value: Box::new(child(*value, children)),
            key: Box::new(child(*key, children)),
        },
        K::HasKey { value, key } => K::HasKey {
            value: Box::new(child(*value, children)),
            key: Box::new(child(*key, children)),
        },
        K::Default { value, fallback } => K::Default {
            value: Box::new(child(*value, children)),
            fallback: Box::new(child(*fallback, children)),
        },
        K::ZipExact { left, right } => K::ZipExact {
            left: Box::new(child(*left, children)),
            right: Box::new(child(*right, children)),
        },
        K::Reduce { operator, array } => K::Reduce {
            operator,
            array: Box::new(child(*array, children)),
        },
        K::PredicateSubject => K::PredicateSubject,
        K::SubjectScope { subject, argument } => K::SubjectScope {
            subject: Box::new(child(*subject, children)),
            argument: PredicateArgument {
                argument: argument.argument,
                body: Box::new(child(*argument.body, children)),
            },
        },
        K::PredicateScope { subject, predicate } => K::PredicateScope {
            subject: Box::new(child(*subject, children)),
            predicate: PredicateArgument {
                argument: predicate.argument,
                body: Box::new(child(*predicate.body, children)),
            },
        },
        K::Filter { array, predicate } => K::Filter {
            array: Box::new(child(*array, children)),
            predicate: PredicateArgument {
                argument: predicate.argument,
                body: Box::new(child(*predicate.body, children)),
            },
        },
        K::Map { array, argument } => K::Map {
            array: Box::new(child(*array, children)),
            argument: PredicateArgument {
                argument: argument.argument,
                body: Box::new(child(*argument.body, children)),
            },
        },
        K::FlatMap { array, argument } => K::FlatMap {
            array: Box::new(child(*array, children)),
            argument: PredicateArgument {
                argument: argument.argument,
                body: Box::new(child(*argument.body, children)),
            },
        },
        K::Head { collection, count } => K::Head {
            collection: Box::new(child(*collection, children)),
            count: Box::new(child(*count, children)),
        },
        K::Tail { collection, count } => K::Tail {
            collection: Box::new(child(*collection, children)),
            count: Box::new(child(*count, children)),
        },
        K::Reverse(collection) => K::Reverse(Box::new(child(*collection, children))),
        K::Check {
            collection,
            operand,
            message,
        } => K::Check {
            collection: Box::new(child(*collection, children)),
            operand: operand.map(|v| Box::new(child(*v, children))),
            message: message.map(|v| Box::new(child(*v, children))),
        },
        K::Windows { items, size } => K::Windows {
            items: Box::new(child(*items, children)),
            size: Box::new(child(*size, children)),
        },
        K::RunsBy { rows, field } => K::RunsBy {
            rows: Box::new(child(*rows, children)),
            field,
        },
        K::Align { left, right, field } => K::Align {
            left: Box::new(child(*left, children)),
            right: Box::new(child(*right, children)),
            field,
        },
        K::Sort {
            array,
            selector,
            descending,
        } => K::Sort {
            array: Box::new(child(*array, children)),
            selector,
            descending,
        },
        K::Extremum {
            operator,
            array,
            field,
        } => K::Extremum {
            operator,
            array: Box::new(child(*array, children)),
            field,
        },
        K::GroupBy { rows, field } => K::GroupBy {
            rows: Box::new(child(*rows, children)),
            field,
        },
        K::Require {
            condition,
            value,
            message,
        } => K::Require {
            condition: Box::new(child(*condition, children)),
            value: Box::new(child(*value, children)),
            message,
        },
        K::Range {
            lower: start,
            upper,
        } => K::Range {
            lower: Box::new(child(*start, children)),
            upper: Box::new(child(*upper, children)),
        },
        K::Queue { items, element } => K::Queue {
            items: Box::new(child(*items, children)),
            element,
        },
        K::PopFront(value) => K::PopFront(Box::new(child(*value, children))),
        K::Break(value) => K::Break(Box::new(child(*value, children))),
        K::Continue(values) => {
            K::Continue(values.into_iter().map(|v| child(v, children)).collect())
        }
        K::PushBackAll { queue, items } => K::PushBackAll {
            queue: Box::new(child(*queue, children)),
            items: Box::new(child(*items, children)),
        },
        K::IdentitySet { element } => K::IdentitySet { element },
        K::Insert { set, item } => K::Insert {
            set: Box::new(child(*set, children)),
            item: Box::new(child(*item, children)),
        },
        K::Fold {
            items,
            initial,
            step,
        } => K::Fold {
            items: Box::new(child(*items, children)),
            initial: Box::new(child(*initial, children)),
            step,
        },
        K::Scan {
            items,
            initial,
            step,
        } => K::Scan {
            items: Box::new(child(*items, children)),
            initial: Box::new(child(*initial, children)),
            step,
        },
        K::Loop { states, body } => K::Loop {
            states: states
                .into_iter()
                .map(|(n, v)| (n, child(v, children)))
                .collect(),
            body: Box::new(child(*body, children)),
        },
    };
    Located::new(value, range)
}
fn map_clauses<Q, R>(
    clauses: Vec<Located<ValueGenerator<Q>>>,
    children: &mut Vec<ValueExpression<Q>>,
) -> Vec<Located<ValueGenerator<R>>> {
    clauses
        .into_iter()
        .map(|clause| {
            Located::new(
                match clause.value {
                    ValueGenerator::In {
                        case,
                        binders,
                        source,
                        filter,
                    } => ValueGenerator::In {
                        case,
                        binders,
                        source: child(source, children),
                        filter: filter.map(|v| child(v, children)),
                    },
                    ValueGenerator::Bind {
                        binder,
                        value,
                        filter,
                    } => ValueGenerator::Bind {
                        binder,
                        value: child(value, children),
                        filter: filter.map(|v| child(v, children)),
                    },
                },
                clause.range,
            )
        })
        .collect()
}
