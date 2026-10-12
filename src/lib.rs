#![doc = include_str!("../README.md")]

mod eval;
mod parser;

use std::fmt;
use std::ops::Range;

pub use eval::{Budget, Host, Limits, NativeInput, NativeResult, NativeValue, Truth, evaluate};
pub use parser::{ArgumentReader, parse_pipeline};

/// The core's recursive parser and evaluator support at most this depth.
/// Hosts may choose a smaller nesting allowance.
pub const MAX_NESTING: usize = 64;

/// A half-open UTF-8 byte range in the query or the host's original source.
/// The host supplies any document identity needed to interpret source ranges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    Query(Range<usize>),
    Source(Range<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryError {
    pub location: Location,
    pub message: String,
}

impl QueryError {
    pub fn query(range: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            location: Location::Query(range),
            message: message.into(),
        }
    }

    pub fn source(range: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            location: Location::Source(range),
            message: message.into(),
        }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.location, self.message)
    }
}

impl std::error::Error for QueryError {}

/// A value with its original query byte range. Typed frontends supply ranges
/// from their own source; the core does not invent host-source coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<T> {
    pub value: T,
    pub range: Range<usize>,
}

impl<T> Located<T> {
    pub fn new(value: T, range: Range<usize>) -> Self {
        Self { value, range }
    }
}

pub type Predicate<L> = Located<PredicateKind<L>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PredicateKind<L> {
    Leaf(L),
    Not(Box<Predicate<L>>),
    Group(Box<Predicate<L>>),
    And(Vec<Predicate<L>>),
    Or(Vec<Predicate<L>>),
}

/// All stages accept a stream. Filter and head return streams; count returns a
/// scalar and must be last. Streams preserve order and duplicate identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage<L> {
    Filter(Predicate<L>),
    Head(usize),
    Count,
}

/// A validated pipeline shared by textual and typed frontends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pipeline<L> {
    stages: Vec<Located<Stage<L>>>,
    range: Range<usize>,
}

impl<L> Pipeline<L> {
    /// Lower typed stages, checking result transitions and predicate depth.
    /// An empty pipeline passes through its explicit input. Depth starts at one
    /// for the pipeline; each predicate node adds one, including groups.
    pub fn new(
        stages: Vec<Located<Stage<L>>>,
        range: Range<usize>,
        nesting_limit: usize,
    ) -> Result<Self, QueryError> {
        let nesting_limit = nesting_limit.min(MAX_NESTING);
        check_depth(1, nesting_limit, &range)?;
        let mut counted = false;
        for stage in &stages {
            if counted {
                return Err(QueryError::query(
                    stage.range.clone(),
                    "a stage cannot follow count",
                ));
            }
            match &stage.value {
                Stage::Filter(predicate) => validate_predicate(predicate, 2, nesting_limit)?,
                Stage::Head(_) => {}
                Stage::Count => counted = true,
            }
        }
        Ok(Self { stages, range })
    }

    pub fn stages(&self) -> &[Located<Stage<L>>] {
        &self.stages
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

fn check_depth(depth: usize, limit: usize, range: &Range<usize>) -> Result<(), QueryError> {
    if depth > limit {
        return Err(QueryError::query(range.clone(), "nesting limit exceeded"));
    }
    Ok(())
}

fn validate_predicate<L>(
    predicate: &Predicate<L>,
    depth: usize,
    limit: usize,
) -> Result<(), QueryError> {
    check_depth(depth, limit, &predicate.range)?;
    match &predicate.value {
        PredicateKind::Leaf(_) => {}
        PredicateKind::Not(child) | PredicateKind::Group(child) => {
            validate_predicate(child, depth + 1, limit)?;
        }
        PredicateKind::And(children) | PredicateKind::Or(children) => {
            if children.is_empty() {
                return Err(QueryError::query(
                    predicate.range.clone(),
                    "a Boolean combination needs at least one predicate",
                ));
            }
            for child in children {
                validate_predicate(child, depth + 1, limit)?;
            }
        }
    }
    Ok(())
}
