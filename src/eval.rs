use std::ops::Range;

use crate::{Location, MAX_NESTING, Pipeline, Predicate, PredicateKind, QueryError, Stage};

/// Hosts choose request allowances. Items is a per-stream ceiling, not an
/// aggregate copy or expansion counter. Nesting is capped by [`MAX_NESTING`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub work: usize,
    pub items: usize,
    pub nesting: usize,
}

/// Caller-owned request state. Input admission, pipeline execution and nested
/// host calls share this budget; successful or failed calls do not refund work.
#[derive(Debug)]
pub struct Budget {
    remaining_work: usize,
    max_items: usize,
    max_nesting: usize,
    depth: usize,
}

impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            remaining_work: limits.work,
            max_items: limits.items,
            max_nesting: limits.nesting.min(MAX_NESTING),
            depth: 0,
        }
    }

    pub fn remaining_work(&self) -> usize {
        self.remaining_work
    }

    /// Charge before a host scan, traversal or other declared unit of work.
    pub fn charge(&mut self, units: usize, location: Location) -> Result<(), QueryError> {
        self.remaining_work = self
            .remaining_work
            .checked_sub(units)
            .ok_or_else(|| QueryError {
                location,
                message: "work limit exceeded".into(),
            })?;
        Ok(())
    }

    /// Check a known stream size before allocating or appending host items.
    pub fn check_items(&self, count: usize, location: Location) -> Result<(), QueryError> {
        if count > self.max_items {
            return Err(QueryError {
                location,
                message: "item limit exceeded".into(),
            });
        }
        Ok(())
    }

    fn within_nesting<T>(
        &mut self,
        location: Location,
        operation: impl FnOnce(&mut Self) -> Result<T, QueryError>,
    ) -> Result<T, QueryError> {
        if self.depth >= self.max_nesting {
            return Err(QueryError {
                location,
                message: "nesting limit exceeded".into(),
            });
        }
        self.depth += 1;
        let result = operation(self);
        self.depth -= 1;
        result
    }
}

/// Explicit native input with retained host state. Construction admits items
/// against the same budget later passed to [`evaluate`], before each push.
#[derive(Debug)]
pub struct NativeInput<I, C> {
    items: Vec<I>,
    context: C,
}

impl<I, C> NativeInput<I, C> {
    pub fn collect(
        context: C,
        items: impl IntoIterator<Item = I>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, QueryError> {
        let mut collected = Vec::new();
        for item in items {
            budget.check_items(collected.len() + 1, location.clone())?;
            budget.charge(1, location.clone())?;
            collected.push(item);
        }
        Ok(Self {
            items: collected,
            context,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NativeValue<I> {
    Stream(Vec<I>),
    Count(usize),
}

/// The context survives both stream and count results. The host owns identity,
/// source lifetimes, limitations and rendering; the core only moves values.
#[derive(Debug)]
pub struct NativeResult<I, C> {
    pub value: NativeValue<I>,
    pub context: C,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    True,
    False,
    Unknown,
}

impl Truth {
    fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
}

/// A static host extension. Leaves inspect native items and retained context.
/// The host charges extra scans to the active budget and returns ordinary
/// failures as errors, reserving Unknown for incomplete predicate knowledge.
pub trait Host<I, C, L> {
    fn evaluate_leaf(
        &self,
        leaf: &L,
        item: &I,
        context: &C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, QueryError>;
}

/// Run the validated pipeline on explicit input. No stage selects a document
/// implicitly. Nested host evaluations pass explicit input and this same
/// mutable budget, retaining the current depth and remaining work.
///
/// Core work units are admitted items, stage visits, filter item visits and
/// visited predicate nodes. Filter keeps only True; head truncates; count is
/// terminal. No stage requires items, context or leaves to implement Clone.
pub fn evaluate<I, C, L>(
    pipeline: &Pipeline<L>,
    host: &impl Host<I, C, L>,
    input: NativeInput<I, C>,
    budget: &mut Budget,
) -> Result<NativeResult<I, C>, QueryError> {
    budget.within_nesting(Location::Query(pipeline.range()), |budget| {
        budget.check_items(input.items.len(), Location::Query(pipeline.range()))?;
        let NativeInput { mut items, context } = input;
        for stage in pipeline.stages() {
            budget.charge(1, Location::Query(stage.range.clone()))?;
            match &stage.value {
                Stage::Filter(predicate) => {
                    let mut retained = Vec::new();
                    for item in items {
                        budget.charge(1, Location::Query(stage.range.clone()))?;
                        if evaluate_predicate(predicate, host, &item, &context, budget)?
                            == Truth::True
                        {
                            retained.push(item);
                        }
                    }
                    items = retained;
                }
                Stage::Head(count) => items.truncate(*count),
                Stage::Count => {
                    return Ok(NativeResult {
                        value: NativeValue::Count(items.len()),
                        context,
                    });
                }
            }
        }
        Ok(NativeResult {
            value: NativeValue::Stream(items),
            context,
        })
    })
}

fn evaluate_predicate<I, C, L>(
    predicate: &Predicate<L>,
    host: &impl Host<I, C, L>,
    item: &I,
    context: &C,
    budget: &mut Budget,
) -> Result<Truth, QueryError> {
    budget.within_nesting(Location::Query(predicate.range.clone()), |budget| {
        budget.charge(1, Location::Query(predicate.range.clone()))?;
        match &predicate.value {
            PredicateKind::Leaf(leaf) => {
                host.evaluate_leaf(leaf, item, context, budget, &predicate.range)
            }
            PredicateKind::Not(child) => {
                Ok(evaluate_predicate(child, host, item, context, budget)?.negate())
            }
            PredicateKind::Group(child) => evaluate_predicate(child, host, item, context, budget),
            PredicateKind::And(children) | PredicateKind::Or(children) => {
                let is_and = matches!(predicate.value, PredicateKind::And(_));
                let decisive = if is_and { Truth::False } else { Truth::True };
                let mut unknown = false;
                for child in children {
                    let truth = evaluate_predicate(child, host, item, context, budget)?;
                    if truth == decisive {
                        return Ok(decisive);
                    }
                    unknown |= truth == Truth::Unknown;
                }
                Ok(if unknown {
                    Truth::Unknown
                } else if is_and {
                    Truth::True
                } else {
                    Truth::False
                })
            }
        }
    })
}
