#![doc = include_str!("../README.md")]

mod eval;
mod parser;
mod program;
mod projection;
mod reader;
mod value;

use std::fmt;
use std::ops::Range;

pub use eval::{
    Budget, EvaluationError, Host, Limits, NativeInput, NativeResult, NativeValue, ProgramLimits,
    Truth, evaluate,
};
pub use parser::{ArgumentReader, parse_pipeline};
pub use program::{
    Bindings, EmissionPolicy, Expression, ExpressionKind, ExpressionStep, ParameterDeclaration,
    ParameterInputs, ParameterType, Program, ProgramHost, Statement, admit_parameters,
    dispose_expression, evaluate_expression, evaluate_program,
};
pub use projection::{
    NativeExpression, NativeOperands, Projection, ProjectionFields, ProjectionHost,
    evaluate_native_value, evaluate_projection, native_expression_kind, native_value_types,
    projection_types,
};
pub use reader::{
    CommonStatement, ReadExpression, ValueReaderHost, read_projection, read_statement_prefix,
    read_value_expression,
};
pub use value::{
    ArrayType, ConstructedValue, KeyAxis, KeyedArrayType, KeyedEntry, NominalMember,
    NominalMemberId, NominalTypeId, SetElementType, ValueBinaryOperator, ValueConstructor,
    ValueExpression, ValueExpressionKind, ValueGenerator, ValueKey, ValueKeyType, ValueQueryHost,
    ValueReducer, ValueType, ValueUnaryOperator, constructed_value_type, copy_constructed_value,
    copy_value_types, count_value, evaluate_query_value, evaluate_value,
    evaluate_value_with_host_validation, free_query_value_names, free_value_names,
    query_value_types, take_one, validate_value_expression, value_expression_types,
};

/// The maximum logical nesting allowance and active nested evaluation calls.
/// Hosts may choose a smaller logical allowance.
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

/// All stages accept a stream. Count and tally are terminal. Hosts supply typed
/// navigation/projection payloads and check stream-kind applicability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage<L, S = ()> {
    Filter(Predicate<L>),
    Head(usize),
    Count,
    Unique,
    Tally,
    Host(S),
}

/// A validated pipeline shared by textual and typed frontends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pipeline<L, S = ()> {
    stages: Vec<Located<Stage<L, S>>>,
    range: Range<usize>,
    nesting: Nesting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nesting {
    Nodes,
    GroupNot,
}

impl Nesting {
    fn pipeline_depth(self) -> usize {
        usize::from(self == Self::Nodes)
    }

    fn predicate_depth<L>(self, predicate: &Predicate<L>) -> usize {
        usize::from(
            self == Self::Nodes
                || matches!(
                    predicate.value,
                    PredicateKind::Group(_) | PredicateKind::Not(_)
                ),
        )
    }
}

impl<L, S> Pipeline<L, S> {
    /// Lower typed stages, checking result transitions and predicate depth.
    /// An empty pipeline passes through its explicit input. Depth starts at one
    /// for the pipeline; each predicate node adds one, including groups.
    pub fn new(
        stages: Vec<Located<Stage<L, S>>>,
        range: Range<usize>,
        nesting_limit: usize,
    ) -> Result<Self, QueryError> {
        Self::lower(stages, range, nesting_limit, Nesting::Nodes)
    }

    /// Lower a frontend that counts only written Group and Not wrappers.
    /// Pipeline, leaf and And/Or nodes consume no logical depth. The core walks
    /// predicates iteratively and separately caps active nested evaluation calls.
    pub fn with_group_not_nesting(
        stages: Vec<Located<Stage<L, S>>>,
        range: Range<usize>,
        nesting_limit: usize,
    ) -> Result<Self, QueryError> {
        Self::lower(stages, range, nesting_limit, Nesting::GroupNot)
    }

    fn lower(
        stages: Vec<Located<Stage<L, S>>>,
        range: Range<usize>,
        nesting_limit: usize,
        nesting: Nesting,
    ) -> Result<Self, QueryError> {
        let nesting_limit = nesting_limit.min(MAX_NESTING);
        check_depth(nesting.pipeline_depth(), nesting_limit, &range)?;
        let mut terminal = None;
        for stage in &stages {
            if let Some(name) = terminal {
                return Err(QueryError::query(
                    stage.range.clone(),
                    format!("a stage cannot follow {name}"),
                ));
            }
            match &stage.value {
                Stage::Filter(predicate) => validate_predicate(predicate, nesting, nesting_limit)?,
                Stage::Head(_) | Stage::Unique | Stage::Host(_) => {}
                Stage::Count => terminal = Some("count"),
                Stage::Tally => terminal = Some("tally"),
            }
        }
        Ok(Self {
            stages,
            range,
            nesting,
        })
    }

    pub fn stages(&self) -> &[Located<Stage<L, S>>] {
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
    nesting: Nesting,
    limit: usize,
) -> Result<(), QueryError> {
    let mut pending = vec![(predicate, nesting.pipeline_depth())];
    while let Some((predicate, parent_depth)) = pending.pop() {
        let depth = parent_depth + nesting.predicate_depth(predicate);
        check_depth(depth, limit, &predicate.range)?;
        match &predicate.value {
            PredicateKind::Leaf(_) => {}
            PredicateKind::Not(child) | PredicateKind::Group(child) => {
                pending.push((child, depth));
            }
            PredicateKind::And(children) | PredicateKind::Or(children) => {
                if children.is_empty() {
                    return Err(QueryError::query(
                        predicate.range.clone(),
                        "a Boolean combination needs at least one predicate",
                    ));
                }
                pending.extend(children.iter().rev().map(|child| (child, depth)));
            }
        }
    }
    Ok(())
}
