use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::ops::Range;

use crate::{
    Located, Location, MAX_NESTING, Nesting, Pipeline, Predicate, PredicateKind, QueryError, Stage,
};

/// Core failures retain their location; native host failures retain their type.
#[derive(Debug, PartialEq, Eq)]
pub enum EvaluationError<E> {
    Core(QueryError),
    Host(E),
}

impl<E> From<QueryError> for EvaluationError<E> {
    fn from(error: QueryError) -> Self {
        Self::Core(error)
    }
}

impl<E: fmt::Display> fmt::Display for EvaluationError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::Host(error) => error.fmt(formatter),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for EvaluationError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::Host(error) => Some(error),
        }
    }
}

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
    calls: usize,
}

impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            remaining_work: limits.work,
            max_items: limits.items,
            max_nesting: limits.nesting.min(MAX_NESTING),
            depth: 0,
            calls: 0,
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

    fn enter_depth(&mut self, units: usize, location: Location) -> Result<(), QueryError> {
        if self.depth + units > self.max_nesting {
            return Err(QueryError {
                location,
                message: "nesting limit exceeded".into(),
            });
        }
        self.depth += units;
        Ok(())
    }

    fn within_pipeline<T, E>(
        &mut self,
        depth: usize,
        location: Location,
        operation: impl FnOnce(&mut Self) -> Result<T, EvaluationError<E>>,
    ) -> Result<T, EvaluationError<E>> {
        if self.calls >= MAX_NESTING {
            return Err(QueryError {
                location,
                message: "nested evaluation call limit exceeded".into(),
            }
            .into());
        }
        self.enter_depth(depth, location)?;
        self.calls += 1;
        let result = operation(self);
        self.calls -= 1;
        self.depth -= depth;
        result
    }
}

/// Explicit native input with retained host state. Collect admits rows; use
/// from_admitted for rows whose host scan already charged the active budget.
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

    /// Retain rows whose host scan/admission work was already charged to this
    /// budget before allocation. Check the stream ceiling without charging again.
    /// The caller must use this same active budget for evaluation.
    pub fn from_admitted(
        context: C,
        items: Vec<I>,
        budget: &Budget,
        location: Location,
    ) -> Result<Self, QueryError> {
        budget.check_items(items.len(), location)?;
        Ok(Self { items, context })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NativeValue<I> {
    Stream(Vec<I>),
    Count(usize),
    Tally(BTreeMap<String, usize>),
}

/// Context survives stream, count and tally results. The host owns identity,
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

/// Static native operations. Every hook receives the retained mutable context
/// and active budget. Use Host failures for domain errors and Core failures for
/// budget refusals; reserve Unknown for incomplete predicate knowledge.
pub trait Host<I, C, L, S = ()> {
    type Error;
    type Key: Eq + Hash;

    /// Check applicability before any row iteration, including empty streams.
    /// Hosts may update stream-kind metadata here. Existing operations need no
    /// default preparation; optional operations require explicit host support.
    fn prepare_stage(
        &self,
        stage: &Located<Stage<L, S>>,
        _context: &mut C,
        _budget: &mut Budget,
    ) -> Result<(), EvaluationError<Self::Error>> {
        match stage.value {
            Stage::Filter(_) | Stage::Head(_) | Stage::Count => Ok(()),
            _ => Err(QueryError::query(stage.range.clone(), "unsupported pipeline stage").into()),
        }
    }

    /// Prepare one row before its Boolean tree. The default charges the existing
    /// filter-row visit. A host that already owns admission may replace it.
    fn prepare_filter_item(
        &self,
        _item: &I,
        _context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<(), EvaluationError<Self::Error>> {
        Ok(budget.charge(1, Location::Query(range.clone()))?)
    }

    fn evaluate_leaf(
        &self,
        leaf: &L,
        item: &I,
        context: &mut C,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<Self::Error>>;

    /// Move native rows through a typed navigation or projection stage. Charge
    /// scans and check known output sizes before allocation. The core checks the
    /// returned stream ceiling without charging admission a second time.
    fn evaluate_stage(
        &self,
        _stage: &S,
        _items: Vec<I>,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<I>, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "unsupported host stage").into())
    }

    /// Supply native identity for stable unique. The core has charged one row
    /// visit; charge only additional key work here (e.g. string bytes beyond one).
    fn identity_key(
        &self,
        _item: &I,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Self::Key, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "unsupported unique identity").into())
    }

    /// Consume a string row for tally, moving owned keys where possible. The
    /// core has charged one row visit; charge only additional key work here.
    fn tally_key(
        &self,
        _item: I,
        _context: &mut C,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<String, EvaluationError<Self::Error>> {
        Err(QueryError::query(range.clone(), "unsupported tally key").into())
    }
}

/// Run a validated pipeline on explicit native input and retain its context.
/// Nested host evaluations use this same budget and preserve active depth.
/// Core work charges stage and predicate-node visits, default filter-row
/// preparation, and one row visit for unique/tally. Host hooks charge scans.
pub fn evaluate<I, C, L, S, H>(
    pipeline: &Pipeline<L, S>,
    host: &H,
    input: NativeInput<I, C>,
    budget: &mut Budget,
) -> Result<NativeResult<I, C>, EvaluationError<H::Error>>
where
    H: Host<I, C, L, S>,
{
    budget.within_pipeline(
        pipeline.nesting.pipeline_depth(),
        Location::Query(pipeline.range()),
        |budget| {
            budget.check_items(input.items.len(), Location::Query(pipeline.range()))?;
            let NativeInput {
                mut items,
                mut context,
            } = input;
            for stage in pipeline.stages() {
                budget.charge(1, Location::Query(stage.range.clone()))?;
                host.prepare_stage(stage, &mut context, budget)?;
                match &stage.value {
                    Stage::Filter(predicate) => {
                        let mut retained = Vec::new();
                        for item in items {
                            host.prepare_filter_item(&item, &mut context, budget, &stage.range)?;
                            if evaluate_predicate(
                                predicate,
                                pipeline.nesting,
                                host,
                                &item,
                                &mut context,
                                budget,
                            )? == Truth::True
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
                    Stage::Unique => {
                        let mut seen = HashSet::new();
                        let mut retained = Vec::new();
                        for item in items {
                            budget.charge(1, Location::Query(stage.range.clone()))?;
                            if seen.insert(host.identity_key(
                                &item,
                                &mut context,
                                budget,
                                &stage.range,
                            )?) {
                                retained.push(item);
                            }
                        }
                        items = retained;
                    }
                    Stage::Tally => {
                        let mut counts = BTreeMap::new();
                        for item in items {
                            budget.charge(1, Location::Query(stage.range.clone()))?;
                            let key = host.tally_key(item, &mut context, budget, &stage.range)?;
                            *counts.entry(key).or_insert(0) += 1;
                        }
                        return Ok(NativeResult {
                            value: NativeValue::Tally(counts),
                            context,
                        });
                    }
                    Stage::Host(payload) => {
                        items = host.evaluate_stage(
                            payload,
                            items,
                            &mut context,
                            budget,
                            &stage.range,
                        )?;
                    }
                }
                budget.check_items(items.len(), Location::Query(stage.range.clone()))?;
            }
            Ok(NativeResult {
                value: NativeValue::Stream(items),
                context,
            })
        },
    )
}

// Predicate frames replace Rust recursion. Logical depth remains active while
// evaluating a child or calling a host, including reentrant host evaluation.
enum PredicateFrame<'a, L> {
    Unary {
        negate: bool,
        depth: usize,
    },
    Boolean {
        remaining: &'a [Predicate<L>],
        conjunction: bool,
        unknown: bool,
        depth: usize,
    },
}

fn evaluate_predicate<I, C, L, S, H>(
    predicate: &Predicate<L>,
    nesting: Nesting,
    host: &H,
    item: &I,
    context: &mut C,
    budget: &mut Budget,
) -> Result<Truth, EvaluationError<H::Error>>
where
    H: Host<I, C, L, S>,
{
    let initial_depth = budget.depth;
    let result = (|| {
        let mut frames = Vec::new();
        let mut next = predicate;
        loop {
            let depth = nesting.predicate_depth(next);
            budget.enter_depth(depth, Location::Query(next.range.clone()))?;
            budget.charge(1, Location::Query(next.range.clone()))?;
            let mut truth = match &next.value {
                PredicateKind::Leaf(leaf) => {
                    host.evaluate_leaf(leaf, item, context, budget, &next.range)?
                }
                PredicateKind::Not(child) | PredicateKind::Group(child) => {
                    frames.push(PredicateFrame::Unary {
                        negate: matches!(next.value, PredicateKind::Not(_)),
                        depth,
                    });
                    next = child;
                    continue;
                }
                PredicateKind::And(children) | PredicateKind::Or(children) => {
                    frames.push(PredicateFrame::Boolean {
                        remaining: &children[1..],
                        conjunction: matches!(next.value, PredicateKind::And(_)),
                        unknown: false,
                        depth,
                    });
                    next = &children[0];
                    continue;
                }
            };
            budget.depth -= depth;
            loop {
                match frames.pop() {
                    None => return Ok(truth),
                    Some(PredicateFrame::Unary { negate, depth }) => {
                        if negate {
                            truth = truth.negate();
                        }
                        budget.depth -= depth;
                    }
                    Some(PredicateFrame::Boolean {
                        remaining,
                        conjunction,
                        mut unknown,
                        depth,
                    }) => {
                        let decisive = if conjunction {
                            Truth::False
                        } else {
                            Truth::True
                        };
                        unknown |= truth == Truth::Unknown;
                        if truth != decisive
                            && let Some((child, remaining)) = remaining.split_first()
                        {
                            frames.push(PredicateFrame::Boolean {
                                remaining,
                                conjunction,
                                unknown,
                                depth,
                            });
                            next = child;
                            break;
                        }
                        if truth != decisive {
                            truth = if unknown {
                                Truth::Unknown
                            } else if conjunction {
                                Truth::True
                            } else {
                                Truth::False
                            };
                        }
                        budget.depth -= depth;
                    }
                }
            }
        }
    })();
    budget.depth = initial_depth;
    result
}
