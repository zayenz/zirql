use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::ops::Range;

use crate::value::identity::{HashIndex, hash_string};
use crate::{
    Located, Location, MAX_NESTING, Nesting, Pipeline, Predicate, PredicateKind, QueryError,
    QueryErrorCategory, Stage,
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

/// Explicit program allowances. Results counts produced or copied native
/// entries across the request; stream_items remains a per-stream ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramLimits {
    pub work: usize,
    pub results: usize,
    pub nesting: usize,
    pub stream_items: usize,
}

/// Caller-owned request state. Input admission, pipeline execution and nested
/// host calls share this budget; successful or failed calls do not refund work.
#[derive(Debug)]
pub struct Budget {
    remaining_work: usize,
    remaining_results: Option<usize>,
    max_items: usize,
    max_nesting: usize,
    depth: usize,
    calls: usize,
}

impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            remaining_work: limits.work,
            remaining_results: None,
            max_items: limits.items,
            max_nesting: limits.nesting.min(MAX_NESTING),
            depth: 0,
            calls: 0,
        }
    }

    pub fn for_program(limits: ProgramLimits) -> Self {
        Self {
            remaining_results: Some(limits.results),
            ..Self::new(Limits {
                work: limits.work,
                items: limits.stream_items,
                nesting: limits.nesting,
            })
        }
    }

    pub fn remaining_work(&self) -> usize {
        self.remaining_work
    }

    /// None denotes a legacy budget without aggregate result accounting.
    pub fn remaining_results(&self) -> Option<usize> {
        self.remaining_results
    }

    /// Admit produced/copied entries before allocation. Legacy budgets leave
    /// this charge inactive. Moving already-admitted values needs no charge.
    pub fn charge_results(&mut self, units: usize, location: Location) -> Result<(), QueryError> {
        if let Some(remaining) = &mut self.remaining_results {
            *remaining = remaining.checked_sub(units).ok_or_else(|| QueryError {
                location,
                message: "result limit exceeded".into(),
                category: QueryErrorCategory::ResultLimit,
            })?;
        }
        Ok(())
    }

    // New program visits/copies must not change legacy frontend work costs.
    pub(crate) fn charge_program_work(
        &mut self,
        units: usize,
        location: Location,
    ) -> Result<(), QueryError> {
        if self.remaining_results.is_some() {
            self.charge(units, location)?;
        }
        Ok(())
    }

    /// Charge before a host scan, traversal or other declared unit of work.
    pub fn charge(&mut self, units: usize, location: Location) -> Result<(), QueryError> {
        self.remaining_work = self
            .remaining_work
            .checked_sub(units)
            .ok_or_else(|| QueryError {
                location,
                message: "work limit exceeded".into(),
                category: QueryErrorCategory::WorkLimit,
            })?;
        Ok(())
    }

    /// Check a known stream size before allocating or appending host items.
    pub fn check_items(&self, count: usize, location: Location) -> Result<(), QueryError> {
        if count > self.max_items {
            return Err(QueryError {
                location,
                message: "item limit exceeded".into(),
                category: QueryErrorCategory::ItemLimit,
            });
        }
        Ok(())
    }

    fn enter_depth(&mut self, units: usize, location: Location) -> Result<(), QueryError> {
        if self.depth + units > self.max_nesting {
            return Err(QueryError {
                location,
                message: "nesting limit exceeded".into(),
                category: QueryErrorCategory::Other,
            });
        }
        self.depth += units;
        Ok(())
    }

    pub(crate) fn check_evaluation_depth(
        &self,
        depth: usize,
        calls: usize,
        location: Location,
    ) -> Result<(), QueryError> {
        if self.calls + calls > MAX_NESTING {
            return Err(QueryError {
                location,
                message: "nested evaluation call limit exceeded".into(),
                category: QueryErrorCategory::Other,
            });
        }
        if self.depth.saturating_add(depth) > self.max_nesting {
            return Err(QueryError {
                location,
                message: "nesting limit exceeded".into(),
                category: QueryErrorCategory::Other,
            });
        }
        Ok(())
    }

    pub(crate) fn remaining_nesting(&self) -> usize {
        self.max_nesting.saturating_sub(self.depth)
    }

    // Host argument readers share the active depth and cannot widen the
    // enclosing reader's allowance by starting another public reader.
    pub(crate) fn within_parsing<T>(
        &mut self,
        depth: usize,
        nesting: usize,
        location: Location,
        operation: impl FnOnce(&mut Self) -> Result<T, QueryError>,
    ) -> Result<T, QueryError> {
        let limit = self.max_nesting;
        self.max_nesting = limit.min(self.depth.saturating_add(nesting));
        let result = self.within_evaluation(depth, location, operation);
        self.max_nesting = limit;
        result
    }

    /// Run nested evaluation with this request's active depth and call guard.
    /// Hosts use this for recursive containers not built by a shared operation,
    /// for example legacy JSON arrays. Count each actual container once; shared
    /// constructors already enter their own level. A zero depth bounds callback
    /// reentry without adding a logical value level. Success and error restore
    /// the prior depth/call state; spent work and result units are not refunded.
    ///
    /// # Errors
    /// Refuses excess nesting/reentry at `location`, or propagates the child's
    /// failure unchanged.
    pub fn within_evaluation<T, E: From<QueryError>>(
        &mut self,
        depth: usize,
        location: Location,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        self.check_evaluation_depth(depth, 1, location)?;
        self.depth += depth;
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
            budget.charge_results(1, location.clone())?;
            collected.push(item);
        }
        Ok(Self {
            items: collected,
            context,
        })
    }

    /// Retain rows whose host scan/admission work was already charged to this
    /// budget before allocation, including results in program mode. Check the
    /// stream ceiling without charging again.
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
    Tally(Vec<(String, usize)>),
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

/// Static native operations. Evaluation hooks receive the retained mutable
/// context and active budget. Work-cost hooks only declare core charges. Use Host
/// failures for domain errors and Core failures for budget refusals; reserve
/// Unknown for incomplete predicate knowledge.
pub trait Host<I, C, L, S = ()> {
    type Error;
    type Key: Eq + Hash;

    /// Work charged at the stage location before stage preparation. The default
    /// is one unit; hosts may weight it by input size and retained context.
    fn stage_work(&self, _items: &[I], _context: &C) -> usize {
        1
    }

    /// Work charged at the stage location before each unique/tally key hook.
    /// The default is one row visit; hosts may return zero when stage or
    /// admission work already accounts for these visits.
    fn collection_item_work(&self) -> usize {
        1
    }

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
            Stage::Filter(_)
            | Stage::Head(_)
            | Stage::Tail(_)
            | Stage::Reverse
            | Stage::Check { .. }
            | Stage::Count => Ok(()),
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

    /// Admit the completed filter result after shared Boolean evaluation.
    /// The default retains known true rows. Hosts that require a complete
    /// observation may refuse final Unknown while allowing decisive alternatives.
    fn filter_truth(
        &self,
        truth: Truth,
        _context: &mut C,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<bool, EvaluationError<Self::Error>> {
        Ok(truth == Truth::True)
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
    /// scans, admit newly produced results and check output sizes before
    /// allocation. Moving admitted rows needs no result charge. The core checks the
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

    /// Supply native identity for stable unique. The core has charged
    /// collection_item_work; charge any additional key work here.
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
    /// core has charged collection_item_work; charge any additional key work here.
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
/// Core work charges the host-declared stage and unique/tally row costs,
/// predicate-node visits, and default filter-row preparation. Host evaluation
/// hooks charge scans and any additional work.
pub fn evaluate<I, C, L, S, H>(
    pipeline: &Pipeline<L, S>,
    host: &H,
    input: NativeInput<I, C>,
    budget: &mut Budget,
) -> Result<NativeResult<I, C>, EvaluationError<H::Error>>
where
    H: Host<I, C, L, S>,
{
    budget.within_evaluation(
        pipeline.nesting.pipeline_depth(),
        Location::Query(pipeline.range()),
        |budget| {
            budget.check_items(input.items.len(), Location::Query(pipeline.range()))?;
            let NativeInput {
                mut items,
                mut context,
            } = input;
            for stage in pipeline.stages() {
                budget.charge(
                    host.stage_work(&items, &context),
                    Location::Query(stage.range.clone()),
                )?;
                host.prepare_stage(stage, &mut context, budget)?;
                match &stage.value {
                    Stage::Filter(predicate) => {
                        let mut retained = Vec::new();
                        for item in items {
                            host.prepare_filter_item(&item, &mut context, budget, &stage.range)?;
                            let truth = evaluate_predicate(
                                predicate,
                                pipeline.nesting,
                                budget,
                                |leaf, budget, range| {
                                    host.evaluate_leaf(leaf, &item, &mut context, budget, range)
                                },
                            )?;
                            if host.filter_truth(truth, &mut context, budget, &predicate.range)? {
                                retained.push(item);
                            }
                        }
                        items = retained;
                    }
                    Stage::Head(count) => items.truncate(*count),
                    Stage::Tail(count) => crate::tail(&mut items, *count),
                    Stage::Reverse => crate::reverse(&mut items),
                    Stage::Check { expected, message } => crate::check_cardinality(
                        items.len(),
                        *expected,
                        message.as_deref(),
                        stage.range.clone(),
                    )?,
                    Stage::Count => {
                        budget.charge_results(1, Location::Query(stage.range.clone()))?;
                        return Ok(NativeResult {
                            value: NativeValue::Count(items.len()),
                            context,
                        });
                    }
                    Stage::Unique => {
                        let mut seen = HashSet::new();
                        let mut retained = Vec::new();
                        for item in items {
                            budget.charge(
                                host.collection_item_work(),
                                Location::Query(stage.range.clone()),
                            )?;
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
                        let location = Location::Query(stage.range.clone());
                        let storage_error = || {
                            QueryError::query(stage.range.clone(), "tally storage limit exceeded")
                                .with_category(QueryErrorCategory::ResultLimit)
                        };
                        budget.charge_results(1, location.clone())?;
                        let mut counts: Vec<(String, usize)> = Vec::new();
                        let mut hashes = HashIndex::default();
                        for item in items {
                            budget.charge(
                                host.collection_item_work(),
                                Location::Query(stage.range.clone()),
                            )?;
                            let key = host.tally_key(item, &mut context, budget, &stage.range)?;
                            budget.charge_program_work(
                                key.len().checked_add(1).ok_or_else(storage_error)?,
                                location.clone(),
                            )?;
                            let hash = hash_string(&key);
                            let mut existing = None;
                            for &index in hashes.candidates(hash) {
                                let previous = &counts[index].0;
                                let work = previous
                                    .len()
                                    .min(key.len())
                                    .checked_add(1)
                                    .ok_or_else(storage_error)?;
                                budget.charge_program_work(work, location.clone())?;
                                if previous == &key {
                                    existing = Some(index);
                                    break;
                                }
                            }
                            if let Some(index) = existing {
                                let count = &mut counts[index].1;
                                *count = count.checked_add(1).ok_or_else(|| {
                                    QueryError::query(stage.range.clone(), "tally count overflow")
                                })?;
                            } else {
                                let count = counts
                                    .len()
                                    .checked_add(1)
                                    .filter(|count| {
                                        count
                                            .checked_mul(std::mem::size_of::<(String, usize)>())
                                            .is_some_and(|bytes| bytes <= isize::MAX as usize)
                                    })
                                    .ok_or_else(storage_error)?;
                                budget.check_items(count, location.clone())?;
                                budget.charge_program_work(1, location.clone())?;
                                // One dictionary entry and its newly produced count.
                                budget.charge_results(2, location.clone())?;
                                hashes.insert_program(hash, counts.len(), budget, &location)?;
                                counts.push((key, 1));
                            }
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

/// Visit every leaf before runtime Boolean short-circuiting can hide a refusal.
pub(crate) fn validate_predicate_leaves<'a, L>(
    predicate: &'a Predicate<L>,
    budget: &mut Budget,
    mut leaf: impl FnMut(&'a L, &mut Budget, &Range<usize>) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    admit_validation_slots::<L>(1, budget, &predicate.range)?;
    let mut pending = vec![(predicate, 1)];
    while let Some((predicate, depth)) = pending.pop() {
        budget.check_evaluation_depth(depth, 1, Location::Query(predicate.range.clone()))?;
        budget.charge(1, Location::Query(predicate.range.clone()))?;
        match &predicate.value {
            PredicateKind::Leaf(value) => budget.within_evaluation(
                depth,
                Location::Query(predicate.range.clone()),
                |budget| leaf(value, budget, &predicate.range),
            )?,
            PredicateKind::Not(child) | PredicateKind::Group(child) => {
                admit_validation_slots::<L>(1, budget, &predicate.range)?;
                pending.push((child, depth + 1));
            }
            PredicateKind::And(children) | PredicateKind::Or(children) => {
                if children.is_empty() {
                    return Err(QueryError::query(
                        predicate.range.clone(),
                        "a Boolean combination needs at least one predicate",
                    ));
                }
                admit_validation_slots::<L>(children.len(), budget, &predicate.range)?;
                pending.extend(children.iter().rev().map(|child| (child, depth + 1)));
            }
        }
    }
    Ok(())
}

fn admit_validation_slots<L>(
    count: usize,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<(), QueryError> {
    if count
        .checked_mul(std::mem::size_of::<(&Predicate<L>, usize)>())
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "predicate allocation size overflow",
        ));
    }
    budget.charge(count, Location::Query(range.clone()))
}

pub(crate) fn evaluate_predicate<L, E: From<QueryError>>(
    predicate: &Predicate<L>,
    nesting: Nesting,
    budget: &mut Budget,
    mut leaf: impl FnMut(&L, &mut Budget, &Range<usize>) -> Result<Truth, E>,
) -> Result<Truth, E> {
    let initial_depth = budget.depth;
    let result = (|| {
        let mut frames = Vec::new();
        let mut next = predicate;
        loop {
            let depth = nesting.predicate_depth(next);
            budget.enter_depth(depth, Location::Query(next.range.clone()))?;
            budget.charge(1, Location::Query(next.range.clone()))?;
            let mut truth = match &next.value {
                PredicateKind::Leaf(guard) => leaf(guard, budget, &next.range)?,
                PredicateKind::Not(child) | PredicateKind::Group(child) => {
                    admit_predicate_frame::<L>(frames.len(), budget, &next.range)?;
                    frames.push(PredicateFrame::Unary {
                        negate: matches!(next.value, PredicateKind::Not(_)),
                        depth,
                    });
                    next = child;
                    continue;
                }
                PredicateKind::And(children) | PredicateKind::Or(children) => {
                    admit_predicate_frame::<L>(frames.len(), budget, &next.range)?;
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

fn admit_predicate_frame<L>(
    count: usize,
    budget: &mut Budget,
    range: &Range<usize>,
) -> Result<(), QueryError> {
    if count
        .checked_add(1)
        .and_then(|count| count.checked_mul(std::mem::size_of::<PredicateFrame<'_, L>>()))
        .is_none_or(|bytes| bytes > isize::MAX as usize)
    {
        return Err(QueryError::query(
            range.clone(),
            "predicate allocation size overflow",
        ));
    }
    budget.charge_program_work(1, Location::Query(range.clone()))
}
