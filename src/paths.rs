use std::{collections::BTreeMap, ops::Range};

use crate::{
    Bindings, Budget, EvaluationError, Located, Location, NativeCapture, Predicate, ProjectionHost,
    QueryError, Truth,
};

/// One shortest native witness per source/sink pair within an explicit edge bound.
/// The typed edge is one host primitive, never an arbitrary query pipeline.
#[derive(Debug, PartialEq, Eq)]
pub struct NativePaths<S, P> {
    pub edge: Located<S>,
    pub target: Predicate<P>,
    pub max_depth: Located<usize>,
    pub range: Range<usize>,
}

/// Domain edges and ordinary target predicates for the shared path evaluator.
/// Edges preserve the singleton subject kind, document owner and identity domain.
/// Hooks charge native scans, copies and allocations with the active budget.
pub trait PathHost<C>: ProjectionHost<C> {
    type Target;

    /// Validate the primitive edge, input kind, native operands and purity.
    fn validate_path_edge(
        &self,
        edge: &Self::Stage,
        input: &Self::Kind,
        scope: &BTreeMap<String, Self::Kind>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), QueryError>;

    /// Validate every target leaf, including leaves in unchosen Boolean branches.
    fn validate_target_leaf(
        &self,
        target: &Self::Target,
        input: &Self::Kind,
        scope: &BTreeMap<String, Self::Kind>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), QueryError>;

    fn evaluate_target_leaf(
        &self,
        target: &Self::Target,
        input: &Self::Value,
        context: &mut C,
        bindings: &Bindings<Self::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<Self::Error>>;

    /// Return admitted singleton neighbors in the host's direct edge order.
    /// Retain unresolved-edge limitations and refuse owner/stale-handle failures.
    fn path_neighbors(
        &self,
        edge: &Self::Stage,
        input: &Self::Value,
        context: &mut C,
        bindings: &Bindings<Self::Value>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<Self::Value>, EvaluationError<Self::Error>>;

    /// Compare actual native identities; the core charges each comparison first.
    fn same_path_identity(
        &self,
        left: &Self::Value,
        right: &Self::Value,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<bool, EvaluationError<Self::Error>>;

    /// Move admitted singleton steps into one ordered native selection. Its kind
    /// must agree with `subject_kind`; preserve sequence shape even at length one.
    fn path_steps(
        &self,
        steps: Vec<Self::Value>,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Value, EvaluationError<Self::Error>>;
}

/// Preflight edge applicability, every target leaf and scope before enumeration.
/// The ordered schema remains usable for an empty native capture collection.
///
/// # Errors
/// Invalid native input, edge, target, scope, effects or limits retain query ranges.
pub fn path_types<C, H: PathHost<C>>(
    paths: &NativePaths<H::Stage, H::Target>,
    host: &H,
    input: &H::Kind,
    scope: &BTreeMap<String, H::Kind>,
    budget: &mut Budget,
) -> Result<NativeCapture<H::Kind>, QueryError> {
    budget.within_evaluation(1, Location::Query(paths.range.clone()), |budget| {
        let subject = host.subject_kind(input, budget, &paths.range)?;
        budget.charge(1, Location::Query(paths.edge.range.clone()))?;
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
        crate::eval::validate_predicate_leaves(&paths.target, budget, |leaf, budget, range| {
            host.validate_target_leaf(leaf, &subject, scope, budget, range)
        })?;
        slots::<(String, H::Kind)>(3, budget, &paths.range)?;
        let mut fields = Vec::with_capacity(3);
        for name in ["source", "sink", "steps"] {
            budget.charge(name.len(), Location::Query(paths.range.clone()))?;
            fields.push((name.to_owned(), subject.clone()));
        }
        Ok(NativeCapture { fields })
    })
}

/// Execute breadth-first native paths with the original request budget.
/// Distinct roots keep incoming order; sinks keep discovery order. First native
/// discovery fixes equal-length ties. Matching sinks can lead to further sinks.
/// A matching root yields `[source]`; no neighbors are requested at the bound.
///
/// # Errors
/// Aggregate program limits are required. Final Unknown targets, exhaustion and
/// native errors fail the whole operation. Empty output means no witness within
/// the bound and does not establish global disconnection.
pub fn evaluate_paths<C, H: PathHost<C>>(
    paths: &NativePaths<H::Stage, H::Target>,
    host: &H,
    input: H::Value,
    context: &mut C,
    bindings: &Bindings<H::Value>,
    budget: &mut Budget,
) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
    if budget.remaining_results().is_none() {
        return Err(QueryError::query(
            paths.range.clone(),
            "native paths require aggregate result limits",
        )
        .into());
    }
    budget.within_evaluation(1, Location::Query(paths.range.clone()), |budget| {
        let input_kind = host.kind(&input, budget, &paths.range)?;
        let scope = crate::projection::scope_kinds(host, bindings, budget, &paths.range)?;
        path_types(paths, host, &input_kind, &scope, budget)?;
        let sources = host.subjects(input, context, budget, &paths.range)?;
        budget.check_items(sources.len(), Location::Query(paths.range.clone()))?;
        slots::<Vec<H::Value>>(1, budget, &paths.range)?;
        let mut rows = Vec::new();
        'sources: for (index, source) in sources.iter().enumerate() {
            for earlier in &sources[..index] {
                budget.charge(1, Location::Query(paths.range.clone()))?;
                if host.same_path_identity(source, earlier, context, budget, &paths.range)? {
                    continue 'sources;
                }
            }
            slots::<Node<H::Value>>(1, budget, &paths.range)?;
            let mut nodes = vec![Node {
                value: host.copy_value(source, context, budget, &paths.range)?,
                depth: 0,
                previous: None,
            }];
            let mut cursor = 0;
            while cursor < nodes.len() {
                budget.charge(1, Location::Query(paths.range.clone()))?;
                let node = &nodes[cursor];
                match crate::eval::evaluate_predicate(
                    &paths.target,
                    crate::Nesting::Nodes,
                    budget,
                    |leaf, budget, range| {
                        host.evaluate_target_leaf(
                            leaf,
                            &node.value,
                            context,
                            bindings,
                            budget,
                            range,
                        )
                    },
                )? {
                    Truth::True => {
                        grow::<H::Value>(rows.len(), budget, &paths.range)?;
                        rows.push(witness(paths, host, &nodes, cursor, context, budget)?);
                    }
                    Truth::False => {}
                    Truth::Unknown => {
                        return Err(QueryError::query(
                            paths.target.range.clone(),
                            "path target is Unknown",
                        )
                        .into());
                    }
                }
                if node.depth < paths.max_depth.value {
                    budget.charge(1, Location::Query(paths.edge.range.clone()))?;
                    let neighbors = host.path_neighbors(
                        &paths.edge.value,
                        &node.value,
                        context,
                        bindings,
                        budget,
                        &paths.edge.range,
                    )?;
                    budget
                        .check_items(neighbors.len(), Location::Query(paths.edge.range.clone()))?;
                    let depth = node.depth + 1; // node.depth < the usize bound
                    'neighbors: for value in neighbors {
                        for visited in &nodes {
                            budget.charge(1, Location::Query(paths.edge.range.clone()))?;
                            if host.same_path_identity(
                                &value,
                                &visited.value,
                                context,
                                budget,
                                &paths.edge.range,
                            )? {
                                continue 'neighbors;
                            }
                        }
                        grow::<Node<H::Value>>(nodes.len(), budget, &paths.range)?;
                        nodes.push(Node {
                            value,
                            depth,
                            previous: Some(cursor),
                        });
                    }
                }
                cursor += 1;
            }
        }
        Ok(rows)
    })
}

struct Node<V> {
    value: V,
    depth: usize,
    previous: Option<usize>,
}

fn witness<C, H: PathHost<C>>(
    paths: &NativePaths<H::Stage, H::Target>,
    host: &H,
    nodes: &[Node<H::Value>],
    sink: usize,
    context: &mut C,
    budget: &mut Budget,
) -> Result<H::Value, EvaluationError<H::Error>> {
    let range = &paths.range;
    let count = nodes[sink].depth.checked_add(1).ok_or_else(|| {
        QueryError::query(paths.max_depth.range.clone(), "path step count overflow")
    })?;
    budget.check_items(count, Location::Query(range.clone()))?;
    slots::<H::Value>(count, budget, range)?;
    let mut steps = Vec::with_capacity(count);
    let mut cursor = Some(sink);
    while let Some(index) = cursor {
        budget.charge(1, Location::Query(range.clone()))?;
        steps.push(host.copy_value(&nodes[index].value, context, budget, range)?);
        cursor = nodes[index].previous;
    }
    steps.reverse();
    slots::<(String, H::Value)>(3, budget, range)?;
    let source = host.copy_value(&nodes[0].value, context, budget, range)?;
    let sink = host.copy_value(&nodes[sink].value, context, budget, range)?;
    let steps = host.path_steps(steps, context, budget, range)?;
    let mut fields = Vec::with_capacity(3);
    for (name, value) in [("source", source), ("sink", sink), ("steps", steps)] {
        budget.charge(name.len(), Location::Query(range.clone()))?;
        fields.push((name.to_owned(), value));
    }
    host.construct_record(fields, context, budget, range)
}

fn grow<T>(count: usize, budget: &mut Budget, range: &Range<usize>) -> Result<(), QueryError> {
    let count = count
        .checked_add(1)
        .ok_or_else(|| QueryError::query(range.clone(), "path allocation size overflow"))?;
    check_slots::<T>(count, range)?;
    budget.check_items(count, Location::Query(range.clone()))?;
    slots::<T>(1, budget, range)
}

fn slots<T>(count: usize, budget: &mut Budget, range: &Range<usize>) -> Result<(), QueryError> {
    check_slots::<T>(count, range)?;
    budget.charge(count, Location::Query(range.clone()))?;
    budget.charge_results(count, Location::Query(range.clone()))
}

fn check_slots<T>(count: usize, range: &Range<usize>) -> Result<(), QueryError> {
    if count
        .checked_mul(std::mem::size_of::<T>())
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "path allocation size overflow",
        ));
    }
    Ok(())
}

/// Release shared Boolean wrappers iteratively. Hosts own opaque edge and leaf
/// payload disposal, including any native operands retained by those payloads.
pub fn dispose_paths<S, P>(paths: NativePaths<S, P>) {
    crate::matching::dispose_predicate(paths.target);
}
