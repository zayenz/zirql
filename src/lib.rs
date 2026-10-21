#![doc = include_str!("../README.md")]

mod composition;
mod constructed_program;
mod construction;
mod eval;
mod existence;
mod fixed_point;
mod functions;
mod mapping;
mod matching;
mod native_collections;
mod ordering;
mod parser;
mod paths;
mod program;
mod projection;
mod reader;
mod rendering;
mod runtime_types;
mod sets;
mod value;
mod worklists;

use std::fmt;
use std::ops::Range;

pub use constructed_program::{ConstructedBinding, ConstructedProgram};
pub use construction::{construct_string_entries, expand_fragments};
pub use eval::{
    Budget, EvaluationError, Host, Limits, NativeInput, NativeResult, NativeValue, ProgramLimits,
    Truth, evaluate,
};
pub use existence::filter_native_exists;
pub use fixed_point::{evaluate_native_fixed_point, ordered_native_equal};
pub use functions::{
    FunctionCall, FunctionDeclaration, FunctionDeclarationRef, FunctionError, FunctionHost,
    FunctionLimits, FunctionParameter, FunctionRequest, NativeFunction,
    NativeFunctionImplementation, PatternFunctionDeclaration, PatternFunctionDeclarationRef,
    PatternParameter, PatternParameterRef, PreparedExpression, PreparedFunctionValue,
    PreparedFunctions, PreparedPatternBinding, PreparedProgram, dispose_functions,
    dispose_pattern_functions, evaluate_functions, evaluate_pattern_functions,
    pattern_function_expression_kind, pattern_function_expression_kind_borrowed,
    pattern_function_value_kind, pattern_function_value_kind_borrowed,
};
pub use mapping::{MapSelection, NativeMap, map_native_by, select_map, sort_map_by_value};
pub use matching::{
    MatchClause, NativeCapture, NativeMatch, dispose_match, evaluate_match,
    evaluate_match_with_arguments, match_types,
};
pub use native_collections::{
    construct_native_dictionary, construct_native_set, lookup_native_dictionary,
    native_set_contains, native_value_hash, native_values_compare, native_values_equal,
    tally_native_values,
};
pub use ordering::{NativeOrder, NativeOrderKey, NativeOrdering, order_native_by};
pub use parser::{ArgumentReader, parse_pipeline};
pub use paths::{
    NativePaths, dispose_paths, evaluate_paths, evaluate_paths_with_arguments, path_types,
};
pub use program::{
    Bindings, EmissionPolicy, Expression, ExpressionKind, ExpressionStep, ParameterDeclaration,
    ParameterInputs, ParameterType, Program, ProgramHost, Statement, admit_parameters,
    dispose_expression, evaluate_expression, evaluate_program, evaluate_program_with,
};
pub use projection::{
    ConstructedResult, NativeArguments, NativeExpression, NativeOperands, NativeRead,
    NativeValueArguments, Projection, ProjectionFields, ProjectionHost, ScopedNativeArgument,
    ScopedNativeValueArguments, evaluate_native_expression, evaluate_native_predicate,
    evaluate_native_predicate_value, evaluate_native_value, evaluate_native_value_with_types,
    evaluate_projection, evaluate_projection_with_types, native_entry_kind, native_expression_kind,
    native_predicate_types, native_predicate_value_types, native_value_types, projection_types,
};
pub use reader::{
    CommonStatement, FieldPipelineSource, ProgramExpression, ProgramRead, ProgramReaderHost,
    ReadExpression, ReadParameter, ReadProgram, ReadStatement, ReadStep, ResolvedFunctionSignature,
    ResolvedPatternFunctionSignature, SignatureType, UnresolvedFunctionDeclaration,
    UnresolvedFunctionParameter, UnresolvedPatternFunctionDeclaration,
    UnresolvedPatternFunctionParameter, ValueReaderHost, ValueSuffix, ValueSuffixKind,
    dispose_native_value, dispose_unresolved_functions, dispose_unresolved_pattern_functions,
    has_collection_syntax, has_common_expression_syntax, has_named_value_postfix,
    has_native_field_pipeline, is_common_control_call, is_common_keyword, is_common_operation_name,
    is_common_value_call, is_unquoted_binding_name, is_value_continuation, lower_value_suffix,
    probe_ordering_call, read_constructed_filter, read_function_call_in_scope,
    read_function_declaration, read_match, read_native_expression, read_native_predicate_argument,
    read_paths, read_pattern, read_pattern_function_declaration, read_pattern_let,
    read_predicate_argument, read_program, read_projection, read_statement_prefix,
    read_unresolved_function_declaration, read_unresolved_pattern_function_declaration,
    read_value_expression, read_value_expression_in_scope, read_value_program, read_value_suffix,
    starts_common_expression,
};
pub use rendering::{
    JsonLayout, JsonRepresentation, RenderContext, RenderHost, RenderView, render_json,
    render_json_value, render_markdown, render_report_markdown, to_json_value,
    to_report_json_value,
};
pub use runtime_types::{
    DeclaredType, OperationSignature, RuntimeTypeHost, ScalarType, TypeFactPolicy,
    admit_constructed, admit_scalar_type, admit_value, checked_invoke, contains_native,
    copy_declaration, declare_constructed, nonnegative_integer_to_usize,
};
pub use sets::{NativeSetOperator, combine_native_set};
pub use value::identity::{compare_constructed_values, constructed_value_hash};
pub use value::{
    ArrayType, ConstructedRead, ConstructedValue, Dictionary, KeyAxis, KeyedArrayType, KeyedEntry,
    NominalMember, NominalMemberId, NominalTypeId, Pattern, PatternKind, PredicateArgument,
    SetElementType, SetElements, SortSelector, ValueArguments, ValueBinaryOperator,
    ValueConstructor, ValueExpression, ValueExpressionKind, ValueExtremum, ValueGenerator,
    ValueKey, ValueKeyType, ValueMatchArm, ValueQueryHost, ValueReducer, ValueType,
    ValueUnaryOperator, constructed_value_type, copy_constructed_value, copy_pattern,
    copy_value_types, count_value, dispose_pattern, evaluate_query_value, evaluate_value,
    evaluate_value_with_host_validation, free_query_value_names, free_value_names, pattern_names,
    query_value_types, take_one, validate_value_expression, value_expression_types,
};
pub use worklists::{
    ComputationalValue, NativeContextId, NativeIdentity, NativeIdentitySet, NativeQueue,
    admit_computational,
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

/// Stable distinctions used by host diagnostic translations.
/// Other failures retain their informative message and location.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryErrorCategory {
    /// A failure without a dedicated host translation.
    Other,
    /// The request-wide work allowance was exhausted.
    WorkLimit,
    /// A native stream exceeded its item ceiling.
    ItemLimit,
    /// The aggregate produced/copied result allowance was exhausted.
    ResultLimit,
    /// Retained for hosts that refuse null; the shared reader accepts absence.
    UnsupportedNull,
    /// A required native match guard or path target remained Unknown.
    UnknownPredicate,
}

/// A core failure with its original location and human-readable message.
/// Hosts translate known categories and forward other messages informatively;
/// they must not infer categories from message text. Equality includes category.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryError {
    pub location: Location,
    pub message: String,
    pub category: QueryErrorCategory,
}

impl QueryError {
    /// Construct a query error in the general [`QueryErrorCategory::Other`] category.
    pub fn query(range: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            location: Location::Query(range),
            message: message.into(),
            category: QueryErrorCategory::Other,
        }
    }

    /// Construct a source error in the general [`QueryErrorCategory::Other`] category.
    pub fn source(range: Range<usize>, message: impl Into<String>) -> Self {
        Self {
            location: Location::Source(range),
            message: message.into(),
            category: QueryErrorCategory::Other,
        }
    }

    /// Assign a category without changing the message or location.
    /// The producer chooses it from the failure, independently of its wording.
    ///
    /// ```
    /// use zirql::{Location, QueryError, QueryErrorCategory};
    ///
    /// let general = QueryError::query(3..7, "work limit exceeded");
    /// assert_eq!(general.category, QueryErrorCategory::Other);
    /// assert_eq!(QueryError::source(3..7, "source failure").category,
    ///            QueryErrorCategory::Other);
    /// let categorized = general.with_category(QueryErrorCategory::WorkLimit);
    /// assert_eq!(categorized.location, Location::Query(3..7));
    /// assert_eq!(categorized.message, "work limit exceeded");
    /// assert_eq!(categorized.category, QueryErrorCategory::WorkLimit);
    /// ```
    pub fn with_category(mut self, category: QueryErrorCategory) -> Self {
        self.category = category;
        self
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
    Tail(usize),
    Reverse,
    Check {
        expected: Option<usize>,
        message: Option<String>,
    },
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
                Stage::Head(_)
                | Stage::Tail(_)
                | Stage::Reverse
                | Stage::Check { .. }
                | Stage::Unique
                | Stage::Host(_) => {}
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

/// Keep the last `count` admitted rows in their original order.
/// This moves existing rows without allocating or admitting results.
pub fn tail<T>(items: &mut Vec<T>, count: usize) {
    let discard = items.len().saturating_sub(count);
    items.drain(..discard);
}

/// Reverse admitted rows without changing their identity or admission.
pub fn reverse<T>(items: &mut [T]) {
    items.reverse();
}

/// Check an exact count, or require a nonempty value when no count is supplied.
/// Hosts determine which values are countable. Success leaves the value untouched.
///
/// # Errors
/// Returns a located failure with expected/actual counts and the optional message.
pub fn check_cardinality(
    actual: usize,
    expected: Option<usize>,
    message: Option<&str>,
    range: Range<usize>,
) -> Result<(), QueryError> {
    let failure = match expected {
        Some(expected) if actual != expected => format!("expected {expected} items, got {actual}"),
        None if actual == 0 => format!("expected at least one item, got {actual}"),
        _ => return Ok(()),
    };
    let message = match message {
        Some(message) => format!("{failure}: {message}"),
        None => failure,
    };
    Err(QueryError::query(range, format!("check failed: {message}")))
}

pub use value::{
    for_each_value_child, for_each_value_child_mut, map_value_payloads, value_child_count,
    walk_value, walk_value_mut,
};

pub use program::{for_each_expression_child, walk_expression};

pub use value::{
    collection_selection_types, collection_subject_types, restore_collection_declaration,
};

pub use composition::{NativeBody, NativeCollectionKey, SelectionShape, collection_value_types};
