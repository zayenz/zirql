use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::mem::size_of;

use crate::{Bindings, Budget, Located, Location, QueryError};

/// Host-supplied enum identity, scoped to one immutable host context per request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NominalTypeId(pub u64);

/// Host-supplied member identity within its nominal type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NominalMemberId(pub u64);

/// A resolved member. Names and declaration order are retained host facts;
/// semantic equality uses only type/member IDs. Budgeted entry points reject
/// contradictory metadata wherever members are compared.
#[derive(Debug)]
pub struct NominalMember {
    pub type_id: NominalTypeId,
    pub member_id: NominalMemberId,
    pub declaration_order: usize,
    pub type_name: String,
    pub member_name: String,
}

impl PartialEq for NominalMember {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id && self.member_id == other.member_id
    }
}
impl Eq for NominalMember {}

/// A single integer or resolved nominal dimension key.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueKey {
    Integer(i64),
    Member(NominalMember),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKeyType {
    Integer,
    Member(NominalTypeId),
}

/// An established dimension, including typed empty axes. Supplied key order is
/// observable and need not be numeric or declaration order.
#[derive(Debug, PartialEq, Eq)]
pub struct KeyAxis {
    pub key_type: ValueKeyType,
    pub keys: Vec<ValueKey>,
}

/// One sparse cell. Native entries must be unique and in row-major axis order.
#[derive(Debug, PartialEq)]
pub struct KeyedEntry {
    pub key: Vec<ValueKey>,
    pub value: ConstructedValue,
}

/// Keyed facts retain rank/dimension identity and possible element types.
/// `None` axes are deferred expression facts only; concrete data always has
/// `Some` established, nonzero-rank axes.
#[derive(Debug, PartialEq, Eq)]
pub struct KeyedArrayType {
    pub axes: Option<Vec<ValueKeyType>>,
    pub elements: Vec<ValueType>,
}

/// Supported homogeneous set identities. Float identities use numeric equality,
/// so positive and negative zero denote the same element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetElementType {
    Boolean,
    Integer,
    Float,
    Member(NominalTypeId),
}

/// Owned constructed data. Arrays have positive one-based positions. Tuples and
/// records retain declaration order; sets have unique elements in type order.
/// Keyed arrays retain nonzero typed axes and sparse row-major entries; supplied
/// axes need not be contiguous or cover a full Cartesian product.
/// Public construction is checked by the budgeted entry points before use.
/// There is deliberately no deep `Clone`; use [`copy_constructed_value`].
#[derive(Debug, PartialEq)]
pub enum ConstructedValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Member(NominalMember),
    KeyedArray {
        axes: Vec<KeyAxis>,
        entries: Vec<KeyedEntry>,
    },
    Array(Vec<Self>),
    Tuple(Vec<Self>),
    Record(Vec<(String, Self)>),
    Set {
        element_type: Option<SetElementType>,
        elements: Vec<Self>,
    },
}

/// Concrete positional facts, including heterogeneous arrays. `Elements`
/// describes possible element types, without claiming a length or positions.
#[derive(Debug, PartialEq, Eq)]
pub enum ArrayType {
    Positions(Vec<ValueType>),
    Elements(Vec<ValueType>),
}

/// Validation facts supplied by projections or discovered from actual data.
/// `Unestablished` admits deferred checking, such as absent access or an empty
/// source's element type. It is never a runtime value or Boolean truth.
#[derive(Debug, PartialEq, Eq)]
pub enum ValueType {
    Boolean,
    Integer,
    Float,
    String,
    Member(NominalTypeId),
    KeyedArray(KeyedArrayType),
    Array(ArrayType),
    Tuple(Vec<Self>),
    Record(Vec<(String, Self)>),
    Set(Option<SetElementType>),
    Unestablished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueUnaryOperator {
    Positive,
    Negative,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Modulo,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    In,
    And,
    Or,
    Concatenate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueConstructor {
    Array,
    Set,
}

/// Closed summaries over fully evaluated positional or keyed arrays.
/// Length counts present values; sum accepts integers and the quantifiers
/// accept fixed Booleans. Empty identities are 0, 0, true and false.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueReducer {
    Length,
    Sum,
    Forall,
    Exists,
}

pub type ValueExpression<Q = ()> = Located<ValueExpressionKind<Q>>;

/// A host-lowered pure expression. Every node retains its query byte range.
/// Hosts reject unlisted surface forms and decimal spellings that round a
/// nonzero literal to zero before creating this representation.
#[derive(Debug, PartialEq)]
pub enum ValueExpressionKind<Q = ()> {
    Literal(ConstructedValue),
    /// A normalized native operand; it runs only when this node is selected.
    NativeQuery(Box<Q>),
    /// Extract exactly one positional child without copying it.
    One(Box<ValueExpression<Q>>),
    Name(String),
    Array(Vec<ValueExpression<Q>>),
    Tuple(Vec<ValueExpression<Q>>),
    Record(Vec<(Located<String>, ValueExpression<Q>)>),
    Set(Vec<ValueExpression<Q>>),
    /// Literal pairs run in written order and normalize to row-major order.
    /// Empty literals require explicit typed axes; generic sparse data is valid.
    KeyedArray {
        axes: Option<Vec<KeyAxis>>,
        entries: Vec<(ValueExpression<Q>, ValueExpression<Q>)>,
    },
    /// Fallback sources establish axes only after zero entries. They run in the
    /// surrounding scope, outside this comprehension's own generator binders.
    KeyedComprehension {
        clauses: Vec<Located<ValueGenerator<Q>>>,
        key: Box<ValueExpression<Q>>,
        head: Box<ValueExpression<Q>>,
        empty_axis_sources: Option<Vec<ValueExpression<Q>>>,
    },
    Unary {
        operator: ValueUnaryOperator,
        operand: Box<ValueExpression<Q>>,
    },
    Binary {
        operator: ValueBinaryOperator,
        left: Box<ValueExpression<Q>>,
        right: Box<ValueExpression<Q>>,
    },
    Field {
        value: Box<ValueExpression<Q>>,
        field: Located<String>,
    },
    TupleAccess {
        value: Box<ValueExpression<Q>>,
        position: Located<usize>,
    },
    ArrayAccess {
        array: Box<ValueExpression<Q>>,
        index: Box<ValueExpression<Q>>,
    },
    IndexSet(Box<ValueExpression<Q>>),
    /// Expose present keyed cells as positional `key`/`value` records. Rank-one
    /// keys are scalars; higher-rank keys are tuples in dimension order.
    Entries(Box<ValueExpression<Q>>),
    /// Pair keyed arrays with identical ordered axes and present full keys.
    /// Retain left axes/keys and move each pair into a two-element tuple cell.
    ZipExact {
        left: Box<ValueExpression<Q>>,
        right: Box<ValueExpression<Q>>,
    },
    /// Evaluate the entire array before scanning its values in stored order.
    Reduce {
        operator: ValueReducer,
        array: Box<ValueExpression<Q>>,
    },
    /// Group positional records by one static field, preserving first-seen keys
    /// and all rows in input order. Keys are homogeneous integers, strings or
    /// members of one nominal type. Results use existing `key`/`items` records.
    /// Missing fields are deferred until execution; known incompatible types
    /// fail preflight. Selector/key errors use `field`'s range.
    GroupBy {
        rows: Box<ValueExpression<Q>>,
        field: Located<String>,
    },
    /// Preflight both operands; run value only after a true fixed Boolean.
    /// A false condition fails at this node with the literal message.
    Require {
        condition: Box<ValueExpression<Q>>,
        value: Box<ValueExpression<Q>>,
        message: Located<String>,
    },
    Range {
        lower: Box<ValueExpression<Q>>,
        upper: Box<ValueExpression<Q>>,
    },
    /// Conditions are checked in order. The final else is mandatory.
    Conditional {
        branches: Vec<(ValueExpression<Q>, ValueExpression<Q>)>,
        otherwise: Box<ValueExpression<Q>>,
    },
    Comprehension {
        constructor: ValueConstructor,
        clauses: Vec<Located<ValueGenerator<Q>>>,
        head: Box<ValueExpression<Q>>,
    },
}

/// Ordered generator clauses. None discards a value without installing a name.
/// Grouped `In` evaluates its source once and varies the last binder fastest.
#[derive(Debug, PartialEq)]
pub enum ValueGenerator<Q = ()> {
    In {
        binders: Vec<Located<Option<String>>>,
        source: ValueExpression<Q>,
        filter: Option<ValueExpression<Q>>,
    },
    Bind {
        binder: Located<Option<String>>,
        value: ValueExpression<Q>,
        filter: Option<ValueExpression<Q>>,
    },
}

/// Native leaves share scalar validation and runtime branch selection. Hosts
/// supply only normalized-query facts/execution and explicit scalar binding reads.
/// Generator locals are visible to the native operand under the same budget.
pub trait ValueQueryHost<Q> {
    type Error: From<QueryError>;

    fn query_types(
        &mut self,
        query: &Q,
        locals: &[(&str, Vec<ValueType>)],
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError>;
    fn evaluate_query(
        &mut self,
        query: &Q,
        locals: &[(&str, ConstructedValue)],
        budget: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error>;
    fn read_binding(
        &mut self,
        name: &str,
        range: &std::ops::Range<usize>,
        budget: &mut Budget,
    ) -> Result<ConstructedValue, Self::Error>;
    fn validate_name(
        &mut self,
        _name: &str,
        _range: &std::ops::Range<usize>,
        _budget: &mut Budget,
    ) -> Result<(), QueryError> {
        Ok(())
    }
}

struct NoQueries<'a> {
    bindings: Option<&'a Bindings<ConstructedValue>>,
}
impl ValueQueryHost<()> for NoQueries<'_> {
    type Error = QueryError;
    fn query_types(
        &mut self,
        _: &(),
        _: &[(&str, Vec<ValueType>)],
        _: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Vec<ValueType>, QueryError> {
        Err(QueryError::query(
            range.clone(),
            "native queries are unavailable in this value expression",
        ))
    }
    fn evaluate_query(
        &mut self,
        _: &(),
        _: &[(&str, ConstructedValue)],
        _: &mut Budget,
    ) -> Result<ConstructedValue, QueryError> {
        Err(QueryError::query(
            0..0,
            "native queries are unavailable in this value expression",
        ))
    }
    fn read_binding(
        &mut self,
        name: &str,
        range: &std::ops::Range<usize>,
        budget: &mut Budget,
    ) -> Result<ConstructedValue, QueryError> {
        copy_constructed_value(
            self.bindings
                .expect("runtime value bindings")
                .get(name, range.clone())?,
            budget,
            Location::Query(range.clone()),
        )
    }
}

/// Infer every possible result type, including schemas with no runtime rows.
///
/// # Errors
/// Scope, type and budget refusals retain their expression locations.
pub fn value_expression_types(
    expression: &ValueExpression,
    binding_types: &BTreeMap<String, ValueType>,
    budget: &mut Budget,
) -> Result<Vec<ValueType>, QueryError> {
    Analysis::new(
        Globals::Types(binding_types),
        &mut NoQueries { bindings: None },
    )
    .walk(expression, budget)
}

/// Infer shared value facts with registered native query leaves.
///
/// # Errors
/// All branches and local scopes are preflighted before native execution.
pub fn query_value_types<Q, H: ValueQueryHost<Q>>(
    expression: &ValueExpression<Q>,
    binding_types: &BTreeMap<String, Vec<ValueType>>,
    queries: &mut H,
    budget: &mut Budget,
) -> Result<Vec<ValueType>, QueryError> {
    Analysis::new(Globals::Facts(binding_types), queries).walk(expression, budget)
}

/// Collect first-use value names while validating native leaves and local scopes.
///
/// # Errors
/// Returns located scope/type or resource refusals before execution.
pub fn free_query_value_names<Q, H: ValueQueryHost<Q>>(
    expression: &ValueExpression<Q>,
    binding_types: &BTreeMap<String, Vec<ValueType>>,
    queries: &mut H,
    budget: &mut Budget,
) -> Result<Vec<Located<String>>, QueryError> {
    let mut analysis = Analysis::new(Globals::Facts(binding_types), queries);
    analysis.walk(expression, budget)?;
    Ok(analysis.free)
}

/// Preflight and evaluate with native leaves through the ordinary scalar runner.
///
/// # Errors
/// Concrete native failures and located shared refusals retain their owner.
pub fn evaluate_query_value<Q, H: ValueQueryHost<Q>>(
    expression: &ValueExpression<Q>,
    binding_types: &BTreeMap<String, Vec<ValueType>>,
    queries: &mut H,
    budget: &mut Budget,
) -> Result<ConstructedValue, H::Error> {
    query_value_types(expression, binding_types, queries, budget)?;
    run(
        expression,
        queries,
        &mut Vec::new(),
        budget,
        &mut |_, _, _| Ok(()),
    )
}

/// Extract one admitted child without copying it.
///
/// # Errors
/// Zero or multiple children fail at the supplied extraction range.
pub fn take_one<T>(mut values: Vec<T>, range: std::ops::Range<usize>) -> Result<T, QueryError> {
    if values.len() != 1 {
        return Err(QueryError::query(range, "one requires exactly one value"));
    }
    Ok(values.pop().expect("one child"))
}

/// Convert a native count at the explicit constructed-value boundary.
///
/// # Errors
/// Counts beyond i64 or exhausted allowances fail before allocating a value.
pub fn count_value(
    count: usize,
    budget: &mut Budget,
    location: Location,
) -> Result<ConstructedValue, QueryError> {
    let count =
        i64::try_from(count).map_err(|_| error(&location, "count exceeds signed 64-bit range"))?;
    scalar(ConstructedValue::Integer(count), budget, &location)
}

/// Check scopes and all supported type facts, including unchosen branches.
/// Missing access remains a runtime failure if that access is executed.
///
/// # Errors
/// Returns a located scope, type, malformed-value or active-budget refusal.
pub fn validate_value_expression(
    expression: &ValueExpression,
    binding_types: &BTreeMap<String, ValueType>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    Analysis::new(
        Globals::Types(binding_types),
        &mut NoQueries { bindings: None },
    )
    .walk(expression, budget)?;
    Ok(())
}

/// Preflight scopes/types and collect distinct global names at their first-use
/// ranges. Local generator names and discarded binders are omitted.
///
/// # Errors
/// Returns the same refusals as [`validate_value_expression`]. Metadata and
/// copied name bytes spend the supplied budget before allocation.
pub fn free_value_names(
    expression: &ValueExpression,
    binding_types: &BTreeMap<String, ValueType>,
    budget: &mut Budget,
) -> Result<Vec<Located<String>>, QueryError> {
    let mut queries = NoQueries { bindings: None };
    let mut analysis = Analysis::new(Globals::Types(binding_types), &mut queries);
    analysis.walk(expression, budget)?;
    Ok(analysis.free)
}

/// Discover exact concrete type facts while checking externally built values.
/// This includes finite floats, labels, nominal metadata, sets and keyed axes/order.
///
/// # Errors
/// Malformed values and budget exhaustion use the supplied location. Recursive
/// facts, metadata slots and copied labels charge work, even with legacy budgets.
pub fn constructed_value_type(
    value: &ConstructedValue,
    budget: &mut Budget,
    location: Location,
) -> Result<ValueType, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match value {
            ConstructedValue::Boolean(_) => Ok(ValueType::Boolean),
            ConstructedValue::Integer(_) => Ok(ValueType::Integer),
            ConstructedValue::Float(value) => {
                finite(*value, &location)?;
                Ok(ValueType::Float)
            }
            ConstructedValue::String(_) => Ok(ValueType::String),
            ConstructedValue::Member(member) => {
                member_bytes(member, budget, &location)?;
                Ok(ValueType::Member(member.type_id))
            }
            ConstructedValue::KeyedArray { axes, entries } => {
                check_keyed(axes, entries, budget, &location)?;
                let axes = axis_types(axes, budget, &location)?;
                let mut elements = metadata_vec(entries.len(), budget, &location)?;
                for entry in entries {
                    elements.push(constructed_value_type(
                        &entry.value,
                        budget,
                        location.clone(),
                    )?);
                }
                Ok(ValueType::KeyedArray(KeyedArrayType {
                    axes: Some(axes),
                    elements,
                }))
            }
            ConstructedValue::Array(values) | ConstructedValue::Tuple(values) => {
                let mut types = metadata_vec(values.len(), budget, &location)?;
                for value in values {
                    types.push(constructed_value_type(value, budget, location.clone())?);
                }
                Ok(if matches!(value, ConstructedValue::Array(_)) {
                    ValueType::Array(ArrayType::Positions(types))
                } else {
                    ValueType::Tuple(types)
                })
            }
            ConstructedValue::Record(fields) => {
                check_value_labels(fields, budget, &location)?;
                let mut types = metadata_vec(fields.len(), budget, &location)?;
                for (label, value) in fields {
                    budget.charge(label.len(), location.clone())?;
                    let ty = constructed_value_type(value, budget, location.clone())?;
                    types.push((label.to_owned(), ty));
                }
                Ok(ValueType::Record(types))
            }
            ConstructedValue::Set {
                element_type,
                elements,
            } => {
                check_set(*element_type, elements, budget, &location)?;
                Ok(ValueType::Set(*element_type))
            }
        }
    })
}

/// Validate actual referenced binding types, then evaluate a pure expression.
/// All branches are validated before runtime choice. Execution evaluates only
/// needed Boolean operands and selected conditional branches. The host owns
/// source projections and subsequent rendering with this same budget.
///
/// # Errors
/// Returns located scope/type/access/arithmetic or budget refusals and no partial
/// value. Errors restore active nesting/call state without refunding spending.
pub fn evaluate_value(
    expression: &ValueExpression,
    bindings: &Bindings<ConstructedValue>,
    budget: &mut Budget,
) -> Result<ConstructedValue, QueryError> {
    evaluate_value_with_host_validation(expression, bindings, budget, |_, _, _| Ok(()))
}

/// Evaluate with a trusted host's value policy before runtime parent consumption.
/// The check runs on each successful runtime value inside the active evaluation
/// guard, including source and binding copies. It can inspect nested native
/// values and must charge its work to the supplied budget. Runtime-skipped
/// branches do not run the check; all branches still receive static validation.
/// No check is retained after this evaluation, and generic value policy remains
/// unchanged when callers use [`evaluate_value`].
///
/// # Errors
/// Returns ordinary evaluation or host-check refusals with their locations and
/// no partial value. Nesting and local scopes restore without refunding spending.
pub fn evaluate_value_with_host_validation(
    expression: &ValueExpression,
    bindings: &Bindings<ConstructedValue>,
    budget: &mut Budget,
    mut check_value: impl FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
) -> Result<ConstructedValue, QueryError> {
    let mut queries = NoQueries {
        bindings: Some(bindings),
    };
    let mut analysis = Analysis::new(Globals::Available(bindings), &mut queries);
    analysis.walk(expression, budget)?;
    let mut types = BTreeMap::new();
    for name in analysis.free {
        let location = Location::Query(name.range.clone());
        budget.charge(1, location.clone())?;
        admit_diagnostic("undefined binding: ", &name.value, budget, &location)?;
        let value = bindings.get(&name.value, name.range)?;
        let ty = constructed_value_type(value, budget, location)?;
        types.insert(name.value, ty);
    }
    validate_value_expression(expression, &types, budget)?;
    run(
        expression,
        &mut NoQueries {
            bindings: Some(bindings),
        },
        &mut Vec::new(),
        budget,
        &mut check_value,
    )
}

/// Copy a constructed binding or emission snapshot under the active allowance.
/// A recursive invariant/size scan admits all result units and copied bytes
/// before the allocation walk. Repeated snapshots spend again.
///
/// # Errors
/// Malformed values or budget exhaustion fail at `location`; there is no deep
/// uncharged copy and no refund of completed scan/admission work.
pub fn copy_constructed_value(
    value: &ConstructedValue,
    budget: &mut Budget,
    location: Location,
) -> Result<ConstructedValue, QueryError> {
    let (units, bytes) = copy_size(value, budget, &location)?;
    budget.charge(bytes, location.clone())?;
    budget.charge_results(units, location.clone())?;
    copy_admitted(value, budget, &location)
}

fn error(location: &Location, message: impl Into<String>) -> QueryError {
    QueryError {
        location: location.clone(),
        message: message.into(),
    }
}

// Availability checks can discard a Bindings::get error. Admit its potential
// diagnostic before lookup as well as messages that are actually returned.
fn admit_diagnostic(
    prefix: &str,
    value: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    let len = sum(prefix.len(), value.len(), location)?;
    allocation::<u8>(len, location)?;
    budget.charge(len, location.clone())
}

fn sum(a: usize, b: usize, location: &Location) -> Result<usize, QueryError> {
    a.checked_add(b)
        .ok_or_else(|| error(location, "allocation size overflow"))
}

fn allocation<T>(len: usize, location: &Location) -> Result<(), QueryError> {
    if len
        .checked_mul(size_of::<T>())
        .is_none_or(|size| size > isize::MAX as usize)
    {
        return Err(error(location, "allocation size overflow"));
    }
    Ok(())
}

fn metadata_vec<T>(
    len: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<T>, QueryError> {
    allocation::<T>(len, location)?;
    budget.charge(len, location.clone())?;
    Ok(Vec::with_capacity(len))
}

fn push_metadata<T>(
    values: &mut Vec<T>,
    value: T,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    allocation::<T>(sum(values.len(), 1, location)?, location)?;
    budget.charge(1, location.clone())?;
    values.push(value);
    Ok(())
}

fn admit_container<T>(
    len: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<T>, QueryError> {
    allocation::<T>(len, location)?;
    budget.charge(sum(len, 1, location)?, location.clone())?;
    budget.charge_results(sum(len, 1, location)?, location.clone())?;
    Ok(Vec::with_capacity(len))
}

fn finite(value: f64, location: &Location) -> Result<(), QueryError> {
    if !value.is_finite() {
        return Err(error(location, "nonfinite float"));
    }
    Ok(())
}

fn member_bytes(
    member: &NominalMember,
    budget: &mut Budget,
    location: &Location,
) -> Result<usize, QueryError> {
    allocation::<u8>(member.type_name.len(), location)?;
    allocation::<u8>(member.member_name.len(), location)?;
    let bytes = sum(member.type_name.len(), member.member_name.len(), location)?;
    budget.charge(sum(1, bytes, location)?, location.clone())?;
    Ok(bytes)
}

fn check_member_pair(
    left: &NominalMember,
    right: &NominalMember,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.charge(1, location.clone())?;
    if left.type_id != right.type_id {
        return Ok(());
    }
    budget.charge(
        left.type_name.len().min(right.type_name.len()),
        location.clone(),
    )?;
    if left.type_name != right.type_name {
        return Err(error(location, "inconsistent nominal type name"));
    }
    if left.member_id == right.member_id {
        budget.charge(
            left.member_name.len().min(right.member_name.len()),
            location.clone(),
        )?;
        if left.declaration_order != right.declaration_order
            || left.member_name != right.member_name
        {
            return Err(error(location, "inconsistent nominal member metadata"));
        }
    } else if left.declaration_order == right.declaration_order {
        return Err(error(
            location,
            "distinct nominal members share declaration order",
        ));
    }
    Ok(())
}

fn copy_member_admitted(member: &NominalMember) -> NominalMember {
    NominalMember {
        type_id: member.type_id,
        member_id: member.member_id,
        declaration_order: member.declaration_order,
        type_name: member.type_name.to_owned(),
        member_name: member.member_name.to_owned(),
    }
}

fn key_value(key: ValueKey) -> ConstructedValue {
    match key {
        ValueKey::Integer(value) => ConstructedValue::Integer(value),
        ValueKey::Member(member) => ConstructedValue::Member(member),
    }
}

fn key_type(key: &ValueKey) -> ValueKeyType {
    match key {
        ValueKey::Integer(_) => ValueKeyType::Integer,
        ValueKey::Member(member) => ValueKeyType::Member(member.type_id),
    }
}

fn key_set_type(ty: ValueKeyType) -> SetElementType {
    match ty {
        ValueKeyType::Integer => SetElementType::Integer,
        ValueKeyType::Member(id) => SetElementType::Member(id),
    }
}

fn compare_keys(
    left: &ValueKey,
    right: &ValueKey,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    budget.charge(1, location.clone())?;
    match (left, right) {
        (ValueKey::Integer(a), ValueKey::Integer(b)) => Ok(a.cmp(b)),
        (ValueKey::Member(a), ValueKey::Member(b)) if a.type_id == b.type_id => {
            check_member_pair(a, b, budget, location)?;
            Ok(if a.member_id == b.member_id {
                Ordering::Equal
            } else {
                a.declaration_order.cmp(&b.declaration_order)
            })
        }
        _ => Err(error(location, "incompatible keyed dimension type")),
    }
}

fn keys_size(
    keys: &[ValueKey],
    budget: &mut Budget,
    location: &Location,
) -> Result<(usize, usize), QueryError> {
    allocation::<ValueKey>(keys.len(), location)?;
    let units = keys
        .len()
        .checked_mul(2)
        .ok_or_else(|| error(location, "key size overflow"))?;
    let mut bytes = 0;
    for key in keys {
        budget.charge(1, location.clone())?;
        if let ValueKey::Member(member) = key {
            bytes = sum(bytes, member_bytes(member, budget, location)?, location)?;
        }
    }
    Ok((units, bytes))
}

fn copy_keys_admitted(
    keys: &[ValueKey],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueKey>, QueryError> {
    let mut copied = Vec::with_capacity(keys.len());
    for key in keys {
        budget.charge(1, location.clone())?;
        copied.push(match key {
            ValueKey::Integer(value) => ValueKey::Integer(*value),
            ValueKey::Member(member) => ValueKey::Member(copy_member_admitted(member)),
        });
    }
    Ok(copied)
}

fn copy_key(
    key: &ValueKey,
    budget: &mut Budget,
    location: &Location,
) -> Result<ValueKey, QueryError> {
    budget.charge(1, location.clone())?;
    budget.charge_results(1, location.clone())?;
    Ok(match key {
        ValueKey::Integer(value) => ValueKey::Integer(*value),
        ValueKey::Member(member) => {
            let bytes = member_bytes(member, budget, location)?;
            budget.charge(bytes, location.clone())?;
            ValueKey::Member(copy_member_admitted(member))
        }
    })
}

fn check_axes(
    axes: &[KeyAxis],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    if axes.is_empty() {
        return Err(error(
            location,
            "keyed arrays need nonzero rank and established axes",
        ));
    }
    for axis in axes {
        budget.charge(1, location.clone())?;
        for (index, key) in axis.keys.iter().enumerate() {
            budget.charge(1, location.clone())?;
            if key_type(key) != axis.key_type {
                return Err(error(location, "axis key type mismatch"));
            }
            if let ValueKey::Member(member) = key {
                member_bytes(member, budget, location)?;
            }
            for previous in &axis.keys[..index] {
                if compare_keys(previous, key, budget, location)? == Ordering::Equal {
                    return Err(error(location, "duplicate axis key"));
                }
            }
        }
    }
    let members = axes
        .iter()
        .flat_map(|axis| &axis.keys)
        .filter_map(|key| match key {
            ValueKey::Member(member) => Some(member),
            ValueKey::Integer(_) => None,
        });
    for (index, member) in members.clone().enumerate() {
        for previous in members.clone().take(index) {
            check_member_pair(previous, member, budget, location)?;
        }
    }
    Ok(())
}

fn copy_axes(
    axes: &[KeyAxis],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<KeyAxis>, QueryError> {
    check_axes(axes, budget, location)?;
    allocation::<KeyAxis>(axes.len(), location)?;
    budget.charge(axes.len(), location.clone())?;
    budget.charge_results(axes.len(), location.clone())?;
    let mut copied = Vec::with_capacity(axes.len());
    for axis in axes {
        let (units, bytes) = keys_size(&axis.keys, budget, location)?;
        budget.charge(bytes, location.clone())?;
        budget.charge_results(units, location.clone())?;
        copied.push(KeyAxis {
            key_type: axis.key_type,
            keys: copy_keys_admitted(&axis.keys, budget, location)?,
        });
    }
    Ok(copied)
}

fn copy_axis_types(
    types: &[ValueKeyType],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueKeyType>, QueryError> {
    if types.is_empty() {
        return Err(error(location, "keyed arrays need nonzero rank"));
    }
    let mut copied = metadata_vec(types.len(), budget, location)?;
    for ty in types {
        budget.charge(1, location.clone())?;
        copied.push(*ty);
    }
    Ok(copied)
}

fn axis_types(
    axes: &[KeyAxis],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueKeyType>, QueryError> {
    let mut types = metadata_vec(axes.len(), budget, location)?;
    for axis in axes {
        budget.charge(1, location.clone())?;
        types.push(axis.key_type);
    }
    Ok(types)
}

fn same_axis_types(
    left: &[ValueKeyType],
    right: &[ValueKeyType],
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.charge(1, location.clone())?;
    if left.len() != right.len() {
        return Ok(false);
    }
    for (a, b) in left.iter().zip(right) {
        budget.charge(1, location.clone())?;
        if a != b {
            return Ok(false);
        }
    }
    Ok(true)
}

fn key_positions(
    key: &[ValueKey],
    axes: &[KeyAxis],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<usize>, QueryError> {
    if key.len() != axes.len() {
        return Err(error(location, "key rank mismatch"));
    }
    let mut positions = metadata_vec(key.len(), budget, location)?;
    for (key, axis) in key.iter().zip(axes) {
        budget.charge(1, location.clone())?;
        if key_type(key) != axis.key_type {
            return Err(error(location, "incompatible keyed dimension type"));
        }
        let mut position = None;
        // Visit every key so contradictory copies cannot hide beyond a match.
        for (index, existing) in axis.keys.iter().enumerate() {
            if compare_keys(key, existing, budget, location)? == Ordering::Equal {
                position = Some(index);
            }
        }
        positions
            .push(position.ok_or_else(|| error(location, "key is absent from established axis"))?);
    }
    Ok(positions)
}

fn compare_positions(
    left: &[usize],
    right: &[usize],
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    for (a, b) in left.iter().zip(right) {
        budget.charge(1, location.clone())?;
        let order = a.cmp(b);
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(Ordering::Equal)
}

fn equal_keys(
    left: &[ValueKey],
    right: &[ValueKey],
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.charge(1, location.clone())?;
    if left.len() != right.len() {
        return Ok(false);
    }
    for (a, b) in left.iter().zip(right) {
        if compare_keys(a, b, budget, location)? != Ordering::Equal {
            return Ok(false);
        }
    }
    Ok(true)
}

fn check_keyed(
    axes: &[KeyAxis],
    entries: &[KeyedEntry],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    check_axes(axes, budget, location)?;
    let mut previous: Option<Vec<usize>> = None;
    for entry in entries {
        budget.charge(1, location.clone())?;
        let positions = key_positions(&entry.key, axes, budget, location)?;
        if let Some(previous) = previous
            && compare_positions(&previous, &positions, budget, location)? != Ordering::Less
        {
            return Err(error(
                location,
                "keyed entries must be unique and in row-major order",
            ));
        }
        previous = Some(positions);
    }
    let members = axes
        .iter()
        .flat_map(|axis| &axis.keys)
        .chain(entries.iter().flat_map(|entry| &entry.key))
        .filter_map(|key| match key {
            ValueKey::Member(member) => Some(member),
            ValueKey::Integer(_) => None,
        });
    for (index, member) in members.clone().enumerate() {
        for previous in members.clone().take(index) {
            check_member_pair(previous, member, budget, location)?;
        }
    }
    Ok(())
}

fn value_keys(
    value: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueKey>, QueryError> {
    let values = match value {
        ConstructedValue::Tuple(values) if !values.is_empty() => values,
        value @ (ConstructedValue::Integer(_) | ConstructedValue::Member(_)) => {
            let mut values = metadata_vec(1, budget, location)?;
            values.push(value);
            values
        }
        _ => {
            return Err(error(
                location,
                "key needs an integer, member or nonempty flat tuple",
            ));
        }
    };
    allocation::<ValueKey>(values.len(), location)?;
    budget.charge(values.len(), location.clone())?;
    budget.charge_results(values.len(), location.clone())?;
    let mut keys = Vec::with_capacity(values.len());
    for value in values {
        budget.charge(1, location.clone())?;
        keys.push(match value {
            ConstructedValue::Integer(value) => ValueKey::Integer(value),
            ConstructedValue::Member(member) => ValueKey::Member(member),
            _ => return Err(error(location, "key components need integers or members")),
        });
    }
    Ok(keys)
}

fn infer_axes(
    entries: &[Located<KeyedEntry>],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<KeyAxis>, QueryError> {
    let Some(first) = entries.first() else {
        return Err(error(location, "unestablished empty keyed axes"));
    };
    if first.value.key.is_empty() {
        return Err(error(location, "keyed arrays need nonzero rank"));
    }
    allocation::<KeyAxis>(first.value.key.len(), location)?;
    budget.charge(first.value.key.len(), location.clone())?;
    budget.charge_results(first.value.key.len(), location.clone())?;
    let mut axes = Vec::with_capacity(first.value.key.len());
    for key in &first.value.key {
        axes.push(KeyAxis {
            key_type: key_type(key),
            keys: Vec::new(),
        });
    }
    for entry in entries {
        let key_location = Location::Query(entry.range.clone());
        if entry.value.key.len() != axes.len() {
            return Err(error(&key_location, "key rank mismatch"));
        }
        for (key, axis) in entry.value.key.iter().zip(&mut axes) {
            if key_type(key) != axis.key_type {
                return Err(error(&key_location, "incompatible keyed dimension type"));
            }
            let mut position = axis.keys.len();
            let mut found = false;
            for (index, existing) in axis.keys.iter().enumerate() {
                match compare_keys(key, existing, budget, &key_location)? {
                    Ordering::Equal => found = true,
                    Ordering::Less if position == axis.keys.len() => position = index,
                    _ => {}
                }
            }
            if !found {
                let key = copy_key(key, budget, &key_location)?;
                allocation::<ValueKey>(sum(axis.keys.len(), 1, &key_location)?, &key_location)?;
                budget.charge(
                    sum(1, axis.keys.len() - position, &key_location)?,
                    key_location.clone(),
                )?;
                budget.charge_results(1, key_location.clone())?;
                axis.keys.insert(position, key);
            }
        }
    }
    Ok(axes)
}

fn finish_keyed(
    axes: Option<Vec<KeyAxis>>,
    mut entries: Vec<Located<KeyedEntry>>,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let axes = match axes {
        Some(axes) => {
            check_axes(&axes, budget, location)?;
            axes
        }
        None => infer_axes(&entries, budget, location)?,
    };
    let mut positions = metadata_vec(entries.len(), budget, location)?;
    for entry in &entries {
        positions.push(key_positions(
            &entry.value.key,
            &axes,
            budget,
            &Location::Query(entry.range.clone()),
        )?);
    }
    // Insertion sorting admits every comparison and movement; sparse values need
    // no Cartesian cardinality or flattened index arithmetic.
    for index in 1..entries.len() {
        let mut cursor = index;
        while cursor > 0 {
            let key_location = Location::Query(entries[cursor].range.clone());
            match compare_positions(
                &positions[cursor - 1],
                &positions[cursor],
                budget,
                &key_location,
            )? {
                Ordering::Equal => return Err(error(&key_location, "duplicate full key")),
                Ordering::Less => break,
                Ordering::Greater => {
                    budget.charge(2, key_location)?;
                    entries.swap(cursor - 1, cursor);
                    positions.swap(cursor - 1, cursor);
                    cursor -= 1;
                }
            }
        }
    }
    let mut values = metadata_vec(entries.len(), budget, location)?;
    for entry in entries {
        values.push(entry.value);
    }
    check_keyed(&axes, &values, budget, location)?;
    Ok(ConstructedValue::KeyedArray {
        axes,
        entries: values,
    })
}

fn source_axis(
    value: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<KeyAxis, QueryError> {
    let (declared, values) = match value {
        ConstructedValue::Array(values) => (None, values),
        ConstructedValue::KeyedArray { entries, .. } => {
            let mut values = metadata_vec(entries.len(), budget, location)?;
            for entry in entries {
                budget.charge(1, location.clone())?;
                values.push(entry.value);
            }
            (None, values)
        }
        ConstructedValue::Set {
            element_type,
            elements,
        } => {
            let ty = match element_type {
                Some(SetElementType::Integer) => Some(ValueKeyType::Integer),
                Some(SetElementType::Member(id)) => Some(ValueKeyType::Member(id)),
                _ => None,
            };
            (ty, elements)
        }
        _ => {
            return Err(error(
                location,
                "empty axis source needs an integer or member array/set",
            ));
        }
    };
    // The fallback list admits its axis slots before allocating the axis vector.
    // This conversion admits only the new key slots; values move by ownership.
    allocation::<ValueKey>(values.len(), location)?;
    budget.charge(values.len(), location.clone())?;
    budget.charge_results(values.len(), location.clone())?;
    let mut keys = Vec::with_capacity(values.len());
    let mut ty = declared;
    for value in values {
        budget.charge(1, location.clone())?;
        let key = match value {
            ConstructedValue::Integer(value) => ValueKey::Integer(value),
            ConstructedValue::Member(member) => ValueKey::Member(member),
            _ => {
                return Err(error(
                    location,
                    "empty axis source needs integer or member elements",
                ));
            }
        };
        if ty.is_some_and(|ty| ty != key_type(&key)) {
            return Err(error(location, "incompatible empty axis source type"));
        }
        ty = Some(key_type(&key));
        keys.push(key);
    }
    let axis = KeyAxis {
        key_type: ty.ok_or_else(|| error(location, "unestablished empty keyed axis"))?,
        keys,
    };
    check_axes(std::slice::from_ref(&axis), budget, location)?;
    Ok(axis)
}

fn scalar_set_type(value: &ConstructedValue) -> Option<SetElementType> {
    match value {
        ConstructedValue::Boolean(_) => Some(SetElementType::Boolean),
        ConstructedValue::Integer(_) => Some(SetElementType::Integer),
        ConstructedValue::Float(_) => Some(SetElementType::Float),
        ConstructedValue::Member(member) => Some(SetElementType::Member(member.type_id)),
        _ => None,
    }
}

fn compare_set_values(
    left: &ConstructedValue,
    right: &ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    budget.charge(1, location.clone())?;
    match (left, right) {
        (ConstructedValue::Boolean(a), ConstructedValue::Boolean(b)) => Ok(a.cmp(b)),
        (ConstructedValue::Integer(a), ConstructedValue::Integer(b)) => Ok(a.cmp(b)),
        (ConstructedValue::Float(a), ConstructedValue::Float(b)) => {
            finite(*a, location)?;
            finite(*b, location)?;
            Ok(a.partial_cmp(b).expect("finite floats are ordered"))
        }
        (ConstructedValue::Member(a), ConstructedValue::Member(b)) => {
            check_member_pair(a, b, budget, location)?;
            if a.type_id != b.type_id {
                return Err(error(location, "incompatible nominal types"));
            }
            Ok(if a.member_id == b.member_id {
                Ordering::Equal
            } else {
                a.declaration_order.cmp(&b.declaration_order)
            })
        }
        _ => Err(error(
            location,
            "set elements must have one supported scalar type",
        )),
    }
}

fn check_set(
    element_type: Option<SetElementType>,
    elements: &[ConstructedValue],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    if element_type.is_none() && !elements.is_empty() {
        return Err(error(location, "a nonempty set needs its element type"));
    }
    for (index, value) in elements.iter().enumerate() {
        budget.charge(1, location.clone())?;
        if scalar_set_type(value) != element_type {
            return Err(error(location, "set element type mismatch"));
        }
        if let ConstructedValue::Float(value) = value {
            finite(*value, location)?;
        }
        if let ConstructedValue::Member(member) = value {
            member_bytes(member, budget, location)?;
            for previous in &elements[..index] {
                if let ConstructedValue::Member(previous) = previous {
                    check_member_pair(previous, member, budget, location)?;
                }
            }
        }
        if index > 0
            && compare_set_values(&elements[index - 1], value, budget, location)? != Ordering::Less
        {
            return Err(error(
                location,
                "set elements must be unique and in type order",
            ));
        }
    }
    Ok(())
}

fn check_value_labels<T>(
    fields: &[(String, T)],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    for (index, (label, _)) in fields.iter().enumerate() {
        for (previous, _) in &fields[..index] {
            budget.charge(
                sum(1, label.len().min(previous.len()), location)?,
                location.clone(),
            )?;
            if label == previous {
                return Err(error(location, "duplicate record label"));
            }
        }
    }
    Ok(())
}

fn copy_size(
    value: &ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<(usize, usize), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        let mut units = 1;
        let mut bytes = 0;
        match value {
            ConstructedValue::Boolean(_) | ConstructedValue::Integer(_) => {}
            ConstructedValue::Float(value) => finite(*value, location)?,
            ConstructedValue::Member(member) => {
                bytes = member_bytes(member, budget, location)?;
            }
            ConstructedValue::KeyedArray { axes, entries } => {
                check_keyed(axes, entries, budget, location)?;
                allocation::<KeyAxis>(axes.len(), location)?;
                allocation::<KeyedEntry>(entries.len(), location)?;
                units = sum(units, sum(axes.len(), entries.len(), location)?, location)?;
                for axis in axes {
                    let (key_units, key_bytes) = keys_size(&axis.keys, budget, location)?;
                    units = sum(units, key_units, location)?;
                    bytes = sum(bytes, key_bytes, location)?;
                }
                for entry in entries {
                    let (key_units, key_bytes) = keys_size(&entry.key, budget, location)?;
                    let (child_units, child_bytes) = copy_size(&entry.value, budget, location)?;
                    units = sum(units, sum(key_units, child_units, location)?, location)?;
                    bytes = sum(bytes, sum(key_bytes, child_bytes, location)?, location)?;
                }
            }
            ConstructedValue::String(value) => {
                allocation::<u8>(value.len(), location)?;
                bytes = value.len();
            }
            ConstructedValue::Array(values)
            | ConstructedValue::Tuple(values)
            | ConstructedValue::Set {
                elements: values, ..
            } => {
                allocation::<ConstructedValue>(values.len(), location)?;
                if let ConstructedValue::Set { element_type, .. } = value {
                    check_set(*element_type, values, budget, location)?;
                }
                units = sum(units, values.len(), location)?;
                for value in values {
                    let (child_units, child_bytes) = copy_size(value, budget, location)?;
                    units = sum(units, child_units, location)?;
                    bytes = sum(bytes, child_bytes, location)?;
                }
            }
            ConstructedValue::Record(fields) => {
                allocation::<(String, ConstructedValue)>(fields.len(), location)?;
                check_value_labels(fields, budget, location)?;
                units = sum(units, fields.len(), location)?;
                for (label, value) in fields {
                    allocation::<u8>(label.len(), location)?;
                    let (child_units, child_bytes) = copy_size(value, budget, location)?;
                    units = sum(units, child_units, location)?;
                    bytes = sum(bytes, sum(label.len(), child_bytes, location)?, location)?;
                }
            }
        }
        Ok((units, bytes))
    })
}

fn copy_admitted(
    value: &ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match value {
            ConstructedValue::Boolean(value) => ConstructedValue::Boolean(*value),
            ConstructedValue::Integer(value) => ConstructedValue::Integer(*value),
            ConstructedValue::Float(value) => ConstructedValue::Float(*value),
            ConstructedValue::String(value) => ConstructedValue::String(value.to_owned()),
            ConstructedValue::Member(member) => {
                ConstructedValue::Member(copy_member_admitted(member))
            }
            ConstructedValue::KeyedArray { axes, entries } => {
                let mut copied_axes = Vec::with_capacity(axes.len());
                for axis in axes {
                    budget.charge(1, location.clone())?;
                    copied_axes.push(KeyAxis {
                        key_type: axis.key_type,
                        keys: copy_keys_admitted(&axis.keys, budget, location)?,
                    });
                }
                let mut copied_entries = Vec::with_capacity(entries.len());
                for entry in entries {
                    budget.charge(1, location.clone())?;
                    copied_entries.push(KeyedEntry {
                        key: copy_keys_admitted(&entry.key, budget, location)?,
                        value: copy_admitted(&entry.value, budget, location)?,
                    });
                }
                ConstructedValue::KeyedArray {
                    axes: copied_axes,
                    entries: copied_entries,
                }
            }
            ConstructedValue::Array(values)
            | ConstructedValue::Tuple(values)
            | ConstructedValue::Set {
                elements: values, ..
            } => {
                let mut copied = Vec::with_capacity(values.len());
                for value in values {
                    copied.push(copy_admitted(value, budget, location)?);
                }
                match value {
                    ConstructedValue::Array(_) => ConstructedValue::Array(copied),
                    ConstructedValue::Tuple(_) => ConstructedValue::Tuple(copied),
                    ConstructedValue::Set { element_type, .. } => ConstructedValue::Set {
                        element_type: *element_type,
                        elements: copied,
                    },
                    _ => unreachable!("matched collection"),
                }
            }
            ConstructedValue::Record(fields) => {
                let mut copied = Vec::with_capacity(fields.len());
                for (label, value) in fields {
                    copied.push((label.to_owned(), copy_admitted(value, budget, location)?));
                }
                ConstructedValue::Record(copied)
            }
        })
    })
}

fn copy_type(
    ty: &ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<ValueType, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match ty {
            ValueType::Boolean => ValueType::Boolean,
            ValueType::Integer => ValueType::Integer,
            ValueType::Float => ValueType::Float,
            ValueType::String => ValueType::String,
            ValueType::Member(id) => ValueType::Member(*id),
            ValueType::KeyedArray(keyed) => {
                let axes = match &keyed.axes {
                    Some(axes) => Some(copy_axis_types(axes, budget, location)?),
                    None => None,
                };
                ValueType::KeyedArray(KeyedArrayType {
                    axes,
                    elements: copy_types(&keyed.elements, budget, location)?,
                })
            }
            ValueType::Unestablished => ValueType::Unestablished,
            ValueType::Set(element_type) => ValueType::Set(*element_type),
            ValueType::Array(array) => {
                let (ArrayType::Positions(entries) | ArrayType::Elements(entries)) = array;
                let entries = copy_types(entries, budget, location)?;
                ValueType::Array(match array {
                    ArrayType::Positions(_) => ArrayType::Positions(entries),
                    ArrayType::Elements(_) => ArrayType::Elements(entries),
                })
            }
            ValueType::Tuple(entries) => ValueType::Tuple(copy_types(entries, budget, location)?),
            ValueType::Record(fields) => {
                check_value_labels(fields, budget, location)?;
                let mut copied = metadata_vec(fields.len(), budget, location)?;
                for (label, ty) in fields {
                    budget.charge(label.len(), location.clone())?;
                    copied.push((label.to_owned(), copy_type(ty, budget, location)?));
                }
                ValueType::Record(copied)
            }
        })
    })
}

fn copy_types(
    types: &[ValueType],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueType>, QueryError> {
    let mut copied = metadata_vec(types.len(), budget, location)?;
    for ty in types {
        copied.push(copy_type(ty, budget, location)?);
    }
    Ok(copied)
}

fn one_type(
    ty: ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueType>, QueryError> {
    let mut types = metadata_vec(1, budget, location)?;
    types.push(ty);
    Ok(types)
}

fn append_types(
    target: &mut Vec<ValueType>,
    types: Vec<ValueType>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    allocation::<ValueType>(sum(target.len(), types.len(), location)?, location)?;
    budget.charge(types.len(), location.clone())?;
    target.extend(types);
    Ok(())
}

enum Globals<'a> {
    Types(&'a BTreeMap<String, ValueType>),
    Facts(&'a BTreeMap<String, Vec<ValueType>>),
    Available(&'a Bindings<ConstructedValue>),
}

struct Analysis<'a, 'h, Q, H> {
    queries: &'h mut H,
    query: std::marker::PhantomData<Q>,
    globals: Globals<'a>,
    locals: Vec<(&'a str, Vec<ValueType>)>,
    free: Vec<Located<String>>,
}

impl<'a, 'h, Q: 'a, H: ValueQueryHost<Q>> Analysis<'a, 'h, Q, H> {
    fn new(globals: Globals<'a>, queries: &'h mut H) -> Self {
        Self {
            globals,
            queries,
            query: std::marker::PhantomData,
            locals: Vec::new(),
            free: Vec::new(),
        }
    }

    fn global_exists(
        &self,
        name: &str,
        range: std::ops::Range<usize>,
        budget: &mut Budget,
    ) -> Result<bool, QueryError> {
        admit_diagnostic(
            "undefined binding: ",
            name,
            budget,
            &Location::Query(range.clone()),
        )?;
        Ok(match self.globals {
            Globals::Types(types) => types.contains_key(name),
            Globals::Facts(types) => types.contains_key(name),
            Globals::Available(bindings) => bindings.get(name, range).is_ok(),
        })
    }

    fn name(
        &mut self,
        name: &str,
        expression: &ValueExpression<Q>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let location = Location::Query(expression.range.clone());
        if name == "_" {
            return Err(error(&location, "discard binder cannot be read"));
        }
        for (local, types) in self.locals.iter().rev() {
            budget.charge(
                sum(1, name.len().min(local.len()), &location)?,
                location.clone(),
            )?;
            if name == *local {
                return copy_types(types, budget, &location);
            }
        }
        self.queries
            .validate_name(name, &expression.range, budget)?;
        admit_diagnostic("undefined binding: ", name, budget, &location)?;
        let types = match self.globals {
            Globals::Types(types) => one_type(
                copy_type(
                    types
                        .get(name)
                        .ok_or_else(|| error(&location, format!("undefined binding: {name}")))?,
                    budget,
                    &location,
                )?,
                budget,
                &location,
            )?,
            Globals::Facts(types) => copy_types(
                types
                    .get(name)
                    .ok_or_else(|| error(&location, format!("undefined binding: {name}")))?,
                budget,
                &location,
            )?,
            Globals::Available(bindings) => {
                bindings.get(name, expression.range.clone())?;
                one_type(ValueType::Unestablished, budget, &location)?
            }
        };
        let mut found = false;
        for previous in &self.free {
            budget.charge(
                sum(1, name.len().min(previous.value.len()), &location)?,
                location.clone(),
            )?;
            if previous.value == name {
                found = true;
                break;
            }
        }
        if !found {
            budget.charge(name.len(), location.clone())?;
            push_metadata(
                &mut self.free,
                Located::new(name.to_owned(), expression.range.clone()),
                budget,
                &location,
            )?;
        }
        Ok(types)
    }

    fn bind(
        &mut self,
        binder: &'a Located<Option<String>>,
        types: &[ValueType],
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        budget.charge(1, Location::Query(binder.range.clone()))?;
        let Some(name) = binder.value.as_deref() else {
            return Ok(());
        };
        let location = Location::Query(binder.range.clone());
        if name == "_" {
            return Err(error(&location, "use None for a discard binder"));
        }
        if self.global_exists(name, binder.range.clone(), budget)? {
            return Err(error(
                &location,
                "binding shadows an existing source or let",
            ));
        }
        for (previous, _) in &self.locals {
            budget.charge(
                sum(1, name.len().min(previous.len()), &location)?,
                location.clone(),
            )?;
            if name == *previous {
                return Err(error(&location, "binding is already defined"));
            }
        }
        let types = copy_types(types, budget, &location)?;
        push_metadata(&mut self.locals, (name, types), budget, &location)
    }

    fn walk(
        &mut self,
        expression: &'a ValueExpression<Q>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, QueryError> {
        let location = Location::Query(expression.range.clone());
        budget.within_evaluation(1, location.clone(), |budget| {
            budget.charge(1, location.clone())?;
            self.node(expression, budget, &location)
        })
    }

    fn node(
        &mut self,
        expression: &'a ValueExpression<Q>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<ValueType>, QueryError> {
        match &expression.value {
            ValueExpressionKind::Literal(value) => {
                let ty = constructed_value_type(value, budget, location.clone())?;
                one_type(ty, budget, location)
            }
            ValueExpressionKind::Name(name) => self.name(name, expression, budget),
            ValueExpressionKind::NativeQuery(query) => {
                self.queries
                    .query_types(query, &self.locals, budget, &expression.range)
            }
            ValueExpressionKind::One(array) => {
                let mut result = Vec::new();
                for ty in self.walk(array, budget)? {
                    match ty {
                        ValueType::Array(
                            ArrayType::Positions(types) | ArrayType::Elements(types),
                        ) => append_types(&mut result, types, budget, location)?,
                        ValueType::Unestablished => {
                            push_metadata(&mut result, ValueType::Unestablished, budget, location)?
                        }
                        _ => return Err(error(location, "one needs a positional array")),
                    }
                }
                if result.is_empty() {
                    push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                }
                Ok(result)
            }
            ValueExpressionKind::Array(children) => {
                let mut entries = metadata_vec(children.len(), budget, location)?;
                let mut exact = true;
                for child in children {
                    let types = self.walk(child, budget)?;
                    exact &= types.len() == 1;
                    entries.push(types);
                }
                let mut types = Vec::new();
                for entries in entries {
                    append_types(&mut types, entries, budget, location)?;
                }
                one_type(
                    ValueType::Array(if exact {
                        ArrayType::Positions(types)
                    } else {
                        ArrayType::Elements(types)
                    }),
                    budget,
                    location,
                )
            }
            ValueExpressionKind::Tuple(children) => {
                let positions = self.positions(children.iter(), budget, location)?;
                let mut types = metadata_vec(positions.len(), budget, location)?;
                for position in positions {
                    types.push(ValueType::Tuple(position));
                }
                Ok(types)
            }
            ValueExpressionKind::Record(fields) => {
                for (index, (label, _)) in fields.iter().enumerate() {
                    for (previous, _) in &fields[..index] {
                        budget.charge(
                            sum(1, label.value.len().min(previous.value.len()), location)?,
                            location.clone(),
                        )?;
                        if label.value == previous.value {
                            return Err(QueryError::query(
                                label.range.clone(),
                                "duplicate record label",
                            ));
                        }
                    }
                }
                let positions =
                    self.positions(fields.iter().map(|(_, value)| value), budget, location)?;
                let mut types = metadata_vec(positions.len(), budget, location)?;
                for position in positions {
                    let mut result = metadata_vec(fields.len(), budget, location)?;
                    for ((label, _), ty) in fields.iter().zip(position) {
                        budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
                        result.push((label.value.to_owned(), ty));
                    }
                    types.push(ValueType::Record(result));
                }
                Ok(types)
            }
            ValueExpressionKind::Set(children) => {
                let mut element_type = None;
                for child in children {
                    merge_set_types(
                        &mut element_type,
                        &self.walk(child, budget)?,
                        budget,
                        location,
                    )?;
                }
                one_type(ValueType::Set(element_type), budget, location)
            }
            ValueExpressionKind::KeyedArray { axes, entries } => {
                let mut facts = None;
                let established = match axes {
                    Some(axes) => {
                        check_axes(axes, budget, location)?;
                        Some(axis_types(axes, budget, location)?)
                    }
                    None => None,
                };
                let mut elements = Vec::new();
                for (key, value) in entries {
                    let key_location = Location::Query(key.range.clone());
                    merge_key_facts(&mut facts, &self.walk(key, budget)?, budget, &key_location)?;
                    if let Some(axes) = &established {
                        check_key_facts(&facts, axes, budget, &key_location)?;
                    }
                    append_types(&mut elements, self.walk(value, budget)?, budget, location)?;
                }
                let axes = match established {
                    Some(axes) => Some(axes),
                    None => established_facts(facts, budget, location)?,
                };
                if entries.is_empty() && axes.is_none() {
                    return Err(error(location, "unestablished empty keyed axes"));
                }
                one_type(
                    ValueType::KeyedArray(KeyedArrayType { axes, elements }),
                    budget,
                    location,
                )
            }
            ValueExpressionKind::KeyedComprehension {
                clauses,
                key,
                head,
                empty_axis_sources,
            } => {
                // Fallbacks belong to the surrounding scope, before our binders.
                let mut fallback = Vec::new();
                if let Some(sources) = empty_axis_sources {
                    if sources.is_empty() {
                        return Err(error(location, "empty axis fallback needs nonzero rank"));
                    }
                    fallback = metadata_vec(sources.len(), budget, location)?;
                    for source in sources {
                        let types = self.walk(source, budget)?;
                        fallback.push(source_key_type(
                            &types,
                            budget,
                            &Location::Query(source.range.clone()),
                        )?);
                    }
                }
                let saved = self.locals.len();
                let result = (|| {
                    self.clauses(clauses, budget)?;
                    let mut facts = None;
                    merge_key_facts(
                        &mut facts,
                        &self.walk(key, budget)?,
                        budget,
                        &Location::Query(key.range.clone()),
                    )?;
                    let elements = self.walk(head, budget)?;
                    if empty_axis_sources.is_some() {
                        check_fallback_facts(&facts, &fallback, budget, location)?;
                    }
                    let axes = established_facts(facts, budget, location)?;
                    one_type(
                        ValueType::KeyedArray(KeyedArrayType { axes, elements }),
                        budget,
                        location,
                    )
                })();
                self.locals.truncate(saved);
                result
            }
            ValueExpressionKind::Unary { operator, operand } => {
                let types = self.walk(operand, budget)?;
                let mut result = metadata_vec(types.len(), budget, location)?;
                for ty in types {
                    budget.charge(1, location.clone())?;
                    match operator {
                        ValueUnaryOperator::Not => {
                            require_type(&ty, ScalarRequirement::Boolean, location)?;
                            result.push(ValueType::Boolean);
                        }
                        ValueUnaryOperator::Positive | ValueUnaryOperator::Negative => {
                            require_type(&ty, ScalarRequirement::Numeric, location)?;
                            result.push(ty);
                        }
                    }
                }
                Ok(result)
            }
            ValueExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.walk(left, budget)?;
                let right = self.walk(right, budget)?;
                let mut result = Vec::new();
                for a in &left {
                    for b in &right {
                        budget.charge(1, location.clone())?;
                        let ty = binary_type(*operator, a, b, budget, location)?;
                        push_metadata(&mut result, ty, budget, location)?;
                    }
                }
                Ok(result)
            }
            ValueExpressionKind::Field { value, field } => {
                let types = self.walk(value, budget)?;
                let mut result = Vec::new();
                for ty in types {
                    let selected = match ty {
                        ValueType::Record(fields) => {
                            let mut selected = None;
                            for (label, ty) in fields {
                                budget.charge(
                                    sum(1, label.len().min(field.value.len()), location)?,
                                    location.clone(),
                                )?;
                                if label == field.value {
                                    selected = Some(ty);
                                }
                            }
                            selected.unwrap_or(ValueType::Unestablished)
                        }
                        ValueType::Unestablished => ValueType::Unestablished,
                        _ => return Err(error(location, "field access needs a record")),
                    };
                    push_metadata(&mut result, selected, budget, location)?;
                }
                Ok(result)
            }
            ValueExpressionKind::TupleAccess { value, position } => {
                let types = self.walk(value, budget)?;
                let mut result = Vec::new();
                for ty in types {
                    budget.charge(1, location.clone())?;
                    let selected = match ty {
                        ValueType::Tuple(entries) => {
                            budget.charge(entries.len().min(position.value), location.clone())?;
                            position
                                .value
                                .checked_sub(1)
                                .and_then(|index| entries.into_iter().nth(index))
                                .unwrap_or(ValueType::Unestablished)
                        }
                        ValueType::Unestablished => ValueType::Unestablished,
                        _ => return Err(error(location, "tuple access needs a tuple")),
                    };
                    push_metadata(&mut result, selected, budget, location)?;
                }
                Ok(result)
            }
            ValueExpressionKind::ArrayAccess { array, index } => {
                let arrays = self.walk(array, budget)?;
                let indices = self.walk(index, budget)?;
                let constant = match index.value {
                    ValueExpressionKind::Literal(ConstructedValue::Integer(value)) => Some(value),
                    _ => None,
                };
                let mut result = Vec::new();
                for array in arrays {
                    match array {
                        ValueType::KeyedArray(keyed) => {
                            let mut facts = None;
                            merge_key_facts(&mut facts, &indices, budget, location)?;
                            if let Some(axes) = &keyed.axes {
                                check_key_facts(&facts, axes, budget, location)?;
                            }
                            append_types(&mut result, keyed.elements, budget, location)?;
                        }
                        ValueType::Array(array) => {
                            for ty in &indices {
                                require_type(ty, ScalarRequirement::Integer, location)?;
                            }
                            match array {
                                _ if constant.is_some_and(|position| position <= 0) => {
                                    push_metadata(
                                        &mut result,
                                        ValueType::Unestablished,
                                        budget,
                                        location,
                                    )?;
                                }
                                ArrayType::Positions(entries) if constant.is_some() => {
                                    let position =
                                        usize::try_from(constant.expect("constant checked")).ok();
                                    budget.charge(
                                        entries.len().min(position.unwrap_or(0)),
                                        location.clone(),
                                    )?;
                                    let ty = position
                                        .and_then(|position| position.checked_sub(1))
                                        .and_then(|index| entries.into_iter().nth(index))
                                        .unwrap_or(ValueType::Unestablished);
                                    push_metadata(&mut result, ty, budget, location)?;
                                }
                                ArrayType::Positions(entries) | ArrayType::Elements(entries) => {
                                    append_types(&mut result, entries, budget, location)?
                                }
                            }
                        }
                        ValueType::Unestablished => {
                            let mut facts = None;
                            merge_key_facts(&mut facts, &indices, budget, location)?;
                            push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                        }
                        _ => return Err(error(location, "array access needs an array")),
                    }
                }
                if result.is_empty() {
                    push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                }
                Ok(result)
            }
            ValueExpressionKind::IndexSet(array) => {
                let mut result = Vec::new();
                for ty in self.walk(array, budget)? {
                    budget.charge(1, location.clone())?;
                    let element_type = match ty {
                        ValueType::Array(_) => Some(SetElementType::Integer),
                        ValueType::KeyedArray(keyed) => match keyed.axes {
                            Some(axes) if axes.len() == 1 => Some(key_set_type(axes[0])),
                            Some(_) => return Err(error(location, "index_set needs rank one")),
                            None => None,
                        },
                        ValueType::Unestablished => None,
                        _ => return Err(error(location, "index_set needs an array")),
                    };
                    push_metadata(&mut result, ValueType::Set(element_type), budget, location)?;
                }
                Ok(result)
            }
            ValueExpressionKind::Entries(array) => {
                let mut result = Vec::new();
                for ty in self.walk(array, budget)? {
                    budget.charge(1, location.clone())?;
                    let keyed = match ty {
                        ValueType::KeyedArray(keyed) => keyed,
                        ValueType::Unestablished => {
                            push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                            continue;
                        }
                        _ => return Err(error(location, "entries needs a keyed array")),
                    };
                    let key = match keyed.axes {
                        Some(axes) => {
                            let mut components = metadata_vec(axes.len(), budget, location)?;
                            for axis in axes {
                                budget.charge(1, location.clone())?;
                                components.push(match axis {
                                    ValueKeyType::Integer => ValueType::Integer,
                                    ValueKeyType::Member(id) => ValueType::Member(id),
                                });
                            }
                            if components.len() == 1 {
                                components.pop().expect("rank one")
                            } else {
                                ValueType::Tuple(components)
                            }
                        }
                        None => ValueType::Unestablished,
                    };
                    let mut records = metadata_vec(keyed.elements.len(), budget, location)?;
                    for value in keyed.elements {
                        let mut fields = metadata_vec(2, budget, location)?;
                        budget.charge("key".len() + "value".len(), location.clone())?;
                        fields.push(("key".into(), copy_type(&key, budget, location)?));
                        fields.push(("value".into(), value));
                        records.push(ValueType::Record(fields));
                    }
                    push_metadata(
                        &mut result,
                        ValueType::Array(ArrayType::Elements(records)),
                        budget,
                        location,
                    )?;
                }
                Ok(result)
            }
            ValueExpressionKind::ZipExact { left, right } => {
                let left = self.walk(left, budget)?;
                let right = self.walk(right, budget)?;
                let mut result = Vec::new();
                for a in &left {
                    for b in &right {
                        let ty = zip_exact_type(a, b, budget, location)?;
                        push_metadata(&mut result, ty, budget, location)?;
                    }
                }
                Ok(result)
            }
            ValueExpressionKind::Reduce { operator, array } => {
                for ty in self.walk(array, budget)? {
                    budget.charge(1, location.clone())?;
                    let elements = match ty {
                        ValueType::Array(
                            ArrayType::Positions(elements) | ArrayType::Elements(elements),
                        ) => elements,
                        ValueType::KeyedArray(keyed) => keyed.elements,
                        ValueType::Unestablished => continue,
                        _ => return Err(error(location, "reducer needs an array")),
                    };
                    if let Some(requirement) = match operator {
                        ValueReducer::Length => None,
                        ValueReducer::Sum => Some(ScalarRequirement::Integer),
                        ValueReducer::Forall | ValueReducer::Exists => {
                            Some(ScalarRequirement::Boolean)
                        }
                    } {
                        for element in elements {
                            budget.charge(1, location.clone())?;
                            require_type(&element, requirement, location)?;
                        }
                    }
                }
                one_type(
                    match operator {
                        ValueReducer::Length | ValueReducer::Sum => ValueType::Integer,
                        ValueReducer::Forall | ValueReducer::Exists => ValueType::Boolean,
                    },
                    budget,
                    location,
                )
            }
            ValueExpressionKind::GroupBy { rows, field } => {
                let selector = Location::Query(field.range.clone());
                let mut result = Vec::new();
                for ty in self.walk(rows, budget)? {
                    budget.charge(1, location.clone())?;
                    let elements = match ty {
                        ValueType::Array(
                            ArrayType::Positions(elements) | ArrayType::Elements(elements),
                        ) => elements,
                        ValueType::Unestablished => {
                            push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                            continue;
                        }
                        _ => return Err(error(location, "group_by needs a positional array")),
                    };
                    let mut key_type = None;
                    for row in &elements {
                        budget.charge(1, location.clone())?;
                        let fields = match row {
                            ValueType::Record(fields) => fields,
                            ValueType::Unestablished => continue,
                            _ => return Err(error(location, "group_by needs record rows")),
                        };
                        for (label, ty) in fields {
                            budget.charge(
                                sum(1, label.len().min(field.value.len()), &selector)?,
                                selector.clone(),
                            )?;
                            if label != &field.value || matches!(ty, ValueType::Unestablished) {
                                continue;
                            }
                            if !matches!(
                                ty,
                                ValueType::Integer | ValueType::String | ValueType::Member(_)
                            ) {
                                return Err(error(
                                    &selector,
                                    "group_by keys need integers, strings or nominal members",
                                ));
                            }
                            if key_type.as_ref().is_some_and(|previous| previous != ty) {
                                return Err(error(
                                    &selector,
                                    "group_by needs homogeneous key types",
                                ));
                            }
                            if key_type.is_none() {
                                key_type = Some(copy_type(ty, budget, &selector)?);
                            }
                        }
                    }
                    let mut groups = Vec::new();
                    if !elements.is_empty() {
                        let mut fields = metadata_vec(2, budget, location)?;
                        budget.charge("key".len() + "items".len(), location.clone())?;
                        fields.push(("key".into(), key_type.unwrap_or(ValueType::Unestablished)));
                        fields.push((
                            "items".into(),
                            ValueType::Array(ArrayType::Elements(elements)),
                        ));
                        push_metadata(&mut groups, ValueType::Record(fields), budget, location)?;
                    }
                    push_metadata(
                        &mut result,
                        ValueType::Array(ArrayType::Elements(groups)),
                        budget,
                        location,
                    )?;
                }
                Ok(result)
            }
            ValueExpressionKind::Require {
                condition,
                value,
                message,
            } => {
                self.condition(condition, budget)?;
                let types = self.walk(value, budget)?;
                allocation::<u8>(message.value.len(), location)?;
                budget.charge(message.value.len(), Location::Query(message.range.clone()))?;
                Ok(types)
            }
            ValueExpressionKind::Range { lower, upper } => {
                for expression in [lower, upper] {
                    for ty in self.walk(expression, budget)? {
                        require_type(&ty, ScalarRequirement::Integer, location)?;
                    }
                }
                one_type(
                    ValueType::Set(Some(SetElementType::Integer)),
                    budget,
                    location,
                )
            }
            ValueExpressionKind::Conditional {
                branches,
                otherwise,
            } => {
                let mut types = Vec::new();
                for (condition, branch) in branches {
                    self.condition(condition, budget)?;
                    append_types(&mut types, self.walk(branch, budget)?, budget, location)?;
                }
                append_types(&mut types, self.walk(otherwise, budget)?, budget, location)?;
                Ok(types)
            }
            ValueExpressionKind::Comprehension {
                constructor,
                clauses,
                head,
            } => {
                let saved = self.locals.len();
                let result = (|| {
                    self.clauses(clauses, budget)?;
                    let types = self.walk(head, budget)?;
                    let ty = match constructor {
                        ValueConstructor::Array => ValueType::Array(ArrayType::Elements(types)),
                        ValueConstructor::Set => {
                            let mut element_type = None;
                            merge_set_types(&mut element_type, &types, budget, location)?;
                            ValueType::Set(element_type)
                        }
                    };
                    one_type(ty, budget, location)
                })();
                self.locals.truncate(saved);
                result
            }
        }
    }

    fn clauses(
        &mut self,
        clauses: &'a [Located<ValueGenerator<Q>>],
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        for clause in clauses {
            let clause_location = Location::Query(clause.range.clone());
            budget.charge(1, clause_location.clone())?;
            match &clause.value {
                ValueGenerator::In {
                    binders,
                    source,
                    filter,
                } => {
                    if binders.is_empty() {
                        return Err(error(&clause_location, "in generator needs a binder"));
                    }
                    let source_types = self.walk(source, budget)?;
                    let mut elements = Vec::new();
                    for ty in source_types {
                        match ty {
                            ValueType::Array(
                                ArrayType::Positions(entries) | ArrayType::Elements(entries),
                            ) => append_types(&mut elements, entries, budget, &clause_location)?,
                            ValueType::KeyedArray(keyed) => append_types(
                                &mut elements,
                                keyed.elements,
                                budget,
                                &clause_location,
                            )?,
                            ValueType::Set(element_type) => push_metadata(
                                &mut elements,
                                set_value_type(element_type),
                                budget,
                                &clause_location,
                            )?,
                            ValueType::Unestablished => push_metadata(
                                &mut elements,
                                ValueType::Unestablished,
                                budget,
                                &clause_location,
                            )?,
                            _ => {
                                return Err(error(
                                    &Location::Query(source.range.clone()),
                                    "in source needs an array or supported set",
                                ));
                            }
                        }
                    }
                    if elements.is_empty() {
                        push_metadata(
                            &mut elements,
                            ValueType::Unestablished,
                            budget,
                            &clause_location,
                        )?;
                    }
                    for binder in binders {
                        self.bind(binder, &elements, budget)?;
                    }
                    if let Some(filter) = filter {
                        self.condition(filter, budget)?;
                    }
                }
                ValueGenerator::Bind {
                    binder,
                    value,
                    filter,
                } => {
                    let types = self.walk(value, budget)?;
                    self.bind(binder, &types, budget)?;
                    if let Some(filter) = filter {
                        self.condition(filter, budget)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn condition(
        &mut self,
        condition: &'a ValueExpression<Q>,
        budget: &mut Budget,
    ) -> Result<(), QueryError> {
        let location = Location::Query(condition.range.clone());
        for ty in self.walk(condition, budget)? {
            budget.charge(1, location.clone())?;
            require_type(&ty, ScalarRequirement::Boolean, &location)?;
        }
        Ok(())
    }

    // Tuple and record positions retain each concrete branch possibility. Admit
    // the bounded product before materializing it; no unknown slot hides a known
    // incompatible type in a later access or recursive equality.
    fn positions(
        &mut self,
        children: impl Iterator<Item = &'a ValueExpression<Q>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<Vec<ValueType>>, QueryError> {
        let mut positions = metadata_vec(1, budget, location)?;
        positions.push(Vec::new());
        for child in children {
            let types = self.walk(child, budget)?;
            positions = extend_type_positions(&positions, &types, budget, location)?;
        }
        Ok(positions)
    }
}

fn zip_exact_type(
    left: &ValueType,
    right: &ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<ValueType, QueryError> {
    let [left, right] = [left, right].map(|ty| match ty {
        ValueType::KeyedArray(keyed) => Ok(Some(keyed)),
        ValueType::Unestablished => Ok(None),
        _ => Err(error(location, "zip_exact needs keyed arrays")),
    });
    let left = left?;
    let right = right?;
    let left_axes = left.and_then(|keyed| keyed.axes.as_deref());
    let right_axes = right.and_then(|keyed| keyed.axes.as_deref());
    if let (Some(a), Some(b)) = (left_axes, right_axes)
        && !same_axis_types(a, b, budget, location)?
    {
        return Err(error(
            location,
            "zip_exact needs identical axis rank and types",
        ));
    }
    let axes = match left_axes.or(right_axes) {
        Some(axes) => Some(copy_axis_types(axes, budget, location)?),
        None => None,
    };
    let deferred = [ValueType::Unestablished];
    let a = left.map_or(deferred.as_slice(), |keyed| keyed.elements.as_slice());
    let b = right.map_or(deferred.as_slice(), |keyed| keyed.elements.as_slice());
    let mut elements = Vec::new();
    for a in a {
        for b in b {
            let mut pair = metadata_vec(2, budget, location)?;
            pair.push(copy_type(a, budget, location)?);
            pair.push(copy_type(b, budget, location)?);
            push_metadata(&mut elements, ValueType::Tuple(pair), budget, location)?;
        }
    }
    Ok(ValueType::KeyedArray(KeyedArrayType { axes, elements }))
}

// Keep known component facts even when one deferred component prevents a public
// complete axis description. This catches rank/identity conflicts in all arms.
fn key_component_type(
    ty: &ValueType,
    location: &Location,
) -> Result<Option<ValueKeyType>, QueryError> {
    match ty {
        ValueType::Integer => Ok(Some(ValueKeyType::Integer)),
        ValueType::Member(id) => Ok(Some(ValueKeyType::Member(*id))),
        ValueType::Unestablished => Ok(None),
        _ => Err(error(location, "key components need integers or members")),
    }
}

fn merge_key_facts(
    facts: &mut Option<Vec<Option<ValueKeyType>>>,
    types: &[ValueType],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    for ty in types {
        budget.charge(1, location.clone())?;
        let components = match ty {
            ValueType::Unestablished => continue,
            ValueType::Tuple(types) if !types.is_empty() => {
                let mut result = metadata_vec(types.len(), budget, location)?;
                for ty in types {
                    budget.charge(1, location.clone())?;
                    result.push(key_component_type(ty, location)?);
                }
                result
            }
            ValueType::Integer | ValueType::Member(_) => {
                let mut result = metadata_vec(1, budget, location)?;
                result.push(key_component_type(ty, location)?);
                result
            }
            _ => {
                return Err(error(
                    location,
                    "key needs an integer, member or nonempty flat tuple",
                ));
            }
        };
        match facts {
            Some(previous) => {
                if previous.len() != components.len() {
                    return Err(error(location, "key rank mismatch"));
                }
                for (previous, ty) in previous.iter_mut().zip(components) {
                    budget.charge(1, location.clone())?;
                    if previous.is_some() && ty.is_some() && *previous != ty {
                        return Err(error(location, "incompatible keyed dimension type"));
                    }
                    if previous.is_none() {
                        *previous = ty;
                    }
                }
            }
            None => *facts = Some(components),
        }
    }
    Ok(())
}

fn check_key_facts(
    facts: &Option<Vec<Option<ValueKeyType>>>,
    axes: &[ValueKeyType],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    if let Some(facts) = facts {
        if facts.len() != axes.len() {
            return Err(error(location, "key rank mismatch"));
        }
        for (fact, axis) in facts.iter().zip(axes) {
            budget.charge(1, location.clone())?;
            if fact.is_some_and(|fact| fact != *axis) {
                return Err(error(location, "incompatible keyed dimension type"));
            }
        }
    }
    Ok(())
}

fn established_facts(
    facts: Option<Vec<Option<ValueKeyType>>>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Option<Vec<ValueKeyType>>, QueryError> {
    let Some(facts) = facts else {
        return Ok(None);
    };
    let mut axes = metadata_vec(facts.len(), budget, location)?;
    for fact in facts {
        budget.charge(1, location.clone())?;
        let Some(fact) = fact else {
            return Ok(None);
        };
        axes.push(fact);
    }
    Ok(Some(axes))
}

fn source_key_type(
    types: &[ValueType],
    budget: &mut Budget,
    location: &Location,
) -> Result<Option<ValueKeyType>, QueryError> {
    let mut result = None;
    for ty in types {
        budget.charge(1, location.clone())?;
        let keys = match ty {
            ValueType::Array(ArrayType::Positions(elements) | ArrayType::Elements(elements))
            | ValueType::KeyedArray(KeyedArrayType { elements, .. }) => {
                let mut key_type = None;
                for ty in elements {
                    budget.charge(1, location.clone())?;
                    let ty = key_component_type(ty, location)?;
                    if key_type.is_some() && ty.is_some() && key_type != ty {
                        return Err(error(location, "incompatible empty axis source type"));
                    }
                    if ty.is_some() {
                        key_type = ty;
                    }
                }
                key_type
            }
            ValueType::Set(Some(SetElementType::Integer)) => Some(ValueKeyType::Integer),
            ValueType::Set(Some(SetElementType::Member(id))) => Some(ValueKeyType::Member(*id)),
            ValueType::Set(None) | ValueType::Unestablished => None,
            _ => {
                return Err(error(
                    location,
                    "empty axis source needs an integer or member array/set",
                ));
            }
        };
        if result.is_some() && keys.is_some() && result != keys {
            return Err(error(location, "incompatible empty axis source type"));
        }
        if keys.is_some() {
            result = keys;
        }
    }
    Ok(result)
}

fn check_fallback_facts(
    facts: &Option<Vec<Option<ValueKeyType>>>,
    fallback: &[Option<ValueKeyType>],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    if let Some(facts) = facts {
        if facts.len() != fallback.len() {
            return Err(error(location, "empty axis fallback rank mismatch"));
        }
        for (fact, fallback) in facts.iter().zip(fallback) {
            budget.charge(1, location.clone())?;
            if fact.is_some() && fallback.is_some() && *fact != *fallback {
                return Err(error(location, "incompatible empty axis fallback type"));
            }
            // Unknown fallback sources do not acquire identity from the key.
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ScalarRequirement {
    Boolean,
    Integer,
    Numeric,
}

fn require_type(
    ty: &ValueType,
    requirement: ScalarRequirement,
    location: &Location,
) -> Result<(), QueryError> {
    let accepted = matches!(ty, ValueType::Unestablished)
        || match requirement {
            ScalarRequirement::Boolean => matches!(ty, ValueType::Boolean),
            ScalarRequirement::Integer => matches!(ty, ValueType::Integer),
            ScalarRequirement::Numeric => matches!(ty, ValueType::Integer | ValueType::Float),
        };
    if !accepted {
        return Err(error(
            location,
            match requirement {
                ScalarRequirement::Boolean => "condition needs a fixed Boolean",
                ScalarRequirement::Integer => "operation needs integers",
                ScalarRequirement::Numeric => "operation needs numeric operands",
            },
        ));
    }
    Ok(())
}

fn set_value_type(ty: Option<SetElementType>) -> ValueType {
    match ty {
        Some(SetElementType::Boolean) => ValueType::Boolean,
        Some(SetElementType::Integer) => ValueType::Integer,
        Some(SetElementType::Float) => ValueType::Float,
        Some(SetElementType::Member(id)) => ValueType::Member(id),
        None => ValueType::Unestablished,
    }
}

fn merge_set_types(
    element_type: &mut Option<SetElementType>,
    types: &[ValueType],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    for ty in types {
        budget.charge(1, location.clone())?;
        let ty = match ty {
            ValueType::Boolean => SetElementType::Boolean,
            ValueType::Integer => SetElementType::Integer,
            ValueType::Float => SetElementType::Float,
            ValueType::Member(id) => SetElementType::Member(*id),
            ValueType::Unestablished => continue,
            _ => {
                return Err(error(
                    location,
                    "set elements must have one supported scalar type",
                ));
            }
        };
        if element_type.is_some_and(|previous| previous != ty) {
            return Err(error(
                location,
                "set elements must have one supported scalar type",
            ));
        }
        *element_type = Some(ty);
    }
    Ok(())
}

fn compatible_types(
    left: &ValueType,
    right: &ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match (left, right) {
            (ValueType::Unestablished, _) | (_, ValueType::Unestablished) => true,
            (ValueType::Boolean, ValueType::Boolean)
            | (ValueType::Integer, ValueType::Integer)
            | (ValueType::Float, ValueType::Float)
            | (ValueType::String, ValueType::String) => true,
            (ValueType::Member(a), ValueType::Member(b)) => a == b,
            (ValueType::KeyedArray(a), ValueType::KeyedArray(b)) => {
                if let (Some(a), Some(b)) = (&a.axes, &b.axes)
                    && !same_axis_types(a, b, budget, location)?
                {
                    return Ok(false);
                }
                compatible_keyed_elements(&a.elements, &b.elements, budget, location)?
            }
            (ValueType::Set(a), ValueType::Set(b)) => a.is_none() || b.is_none() || a == b,
            (ValueType::Tuple(a), ValueType::Tuple(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    if !compatible_types(a, b, budget, location)? {
                        return Ok(false);
                    }
                }
                true
            }
            (ValueType::Array(a), ValueType::Array(b)) => {
                match (a, b) {
                    (ArrayType::Positions(a), ArrayType::Positions(b)) => {
                        for (a, b) in a.iter().zip(b) {
                            if !compatible_types(a, b, budget, location)? {
                                return Ok(false);
                            }
                        }
                    }
                    (
                        ArrayType::Positions(a) | ArrayType::Elements(a),
                        ArrayType::Positions(b) | ArrayType::Elements(b),
                    ) => {
                        for a in a {
                            for b in b {
                                if !compatible_types(a, b, budget, location)? {
                                    return Ok(false);
                                }
                            }
                        }
                    }
                }
                true
            }
            (ValueType::Record(a), ValueType::Record(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (label, ty) in a {
                    let mut other = None;
                    for (other_label, other_ty) in b {
                        budget.charge(
                            sum(1, label.len().min(other_label.len()), location)?,
                            location.clone(),
                        )?;
                        if label == other_label {
                            other = Some(other_ty);
                            break;
                        }
                    }
                    let Some(other) = other else {
                        return Ok(false);
                    };
                    if !compatible_types(ty, other, budget, location)? {
                        return Ok(false);
                    }
                }
                true
            }
            _ => false,
        })
    })
}

// Keyed facts retain possible element types, not correspondence between keys.
// Check each known family here; execution checks the actual matching cells.
fn compatible_keyed_elements(
    left: &[ValueType],
    right: &[ValueType],
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    if left.is_empty() || right.is_empty() {
        return Ok(true);
    }
    for (types, alternatives) in [(left, right), (right, left)] {
        for ty in types {
            let mut compatible = false;
            for alternative in alternatives {
                if compatible_types(ty, alternative, budget, location)? {
                    compatible = true;
                    break;
                }
            }
            if !compatible {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn binary_type(
    operator: ValueBinaryOperator,
    left: &ValueType,
    right: &ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<ValueType, QueryError> {
    match operator {
        ValueBinaryOperator::Add
        | ValueBinaryOperator::Subtract
        | ValueBinaryOperator::Multiply
        | ValueBinaryOperator::Divide
        | ValueBinaryOperator::Less
        | ValueBinaryOperator::LessEqual
        | ValueBinaryOperator::Greater
        | ValueBinaryOperator::GreaterEqual => {
            require_type(left, ScalarRequirement::Numeric, location)?;
            require_type(right, ScalarRequirement::Numeric, location)?;
            Ok(match operator {
                ValueBinaryOperator::Less
                | ValueBinaryOperator::LessEqual
                | ValueBinaryOperator::Greater
                | ValueBinaryOperator::GreaterEqual => ValueType::Boolean,
                ValueBinaryOperator::Divide => ValueType::Float,
                _ if matches!(left, ValueType::Float) || matches!(right, ValueType::Float) => {
                    ValueType::Float
                }
                _ if matches!(left, ValueType::Unestablished)
                    || matches!(right, ValueType::Unestablished) =>
                {
                    ValueType::Unestablished
                }
                _ => ValueType::Integer,
            })
        }
        ValueBinaryOperator::IntegerDivide | ValueBinaryOperator::Modulo => {
            require_type(left, ScalarRequirement::Integer, location)?;
            require_type(right, ScalarRequirement::Integer, location)?;
            Ok(ValueType::Integer)
        }
        ValueBinaryOperator::And | ValueBinaryOperator::Or => {
            require_type(left, ScalarRequirement::Boolean, location)?;
            require_type(right, ScalarRequirement::Boolean, location)?;
            Ok(ValueType::Boolean)
        }
        ValueBinaryOperator::Equal | ValueBinaryOperator::NotEqual => {
            if !compatible_types(left, right, budget, location)? {
                return Err(error(location, "equality needs compatible types"));
            }
            Ok(ValueType::Boolean)
        }
        ValueBinaryOperator::In => {
            let mut element_type = None;
            merge_set_types(
                &mut element_type,
                std::slice::from_ref(left),
                budget,
                location,
            )?;
            match right {
                ValueType::Set(ty)
                    if ty.is_none() || element_type.is_none() || ty == &element_type =>
                {
                    Ok(ValueType::Boolean)
                }
                ValueType::Unestablished => Ok(ValueType::Boolean),
                _ => Err(error(
                    location,
                    "membership needs a compatible supported set",
                )),
            }
        }
        ValueBinaryOperator::Concatenate => match (left, right) {
            (ValueType::String, ValueType::String) => Ok(ValueType::String),
            (ValueType::Array(a), ValueType::Array(b)) => {
                let (ArrayType::Positions(a_entries) | ArrayType::Elements(a_entries)) = a;
                let (ArrayType::Positions(b_entries) | ArrayType::Elements(b_entries)) = b;
                let mut entries = copy_types(a_entries, budget, location)?;
                append_types(
                    &mut entries,
                    copy_types(b_entries, budget, location)?,
                    budget,
                    location,
                )?;
                Ok(ValueType::Array(
                    if matches!((a, b), (ArrayType::Positions(_), ArrayType::Positions(_))) {
                        ArrayType::Positions(entries)
                    } else {
                        ArrayType::Elements(entries)
                    },
                ))
            }
            (ValueType::Unestablished, other) | (other, ValueType::Unestablished)
                if matches!(
                    other,
                    ValueType::Unestablished | ValueType::String | ValueType::Array(_)
                ) =>
            {
                Ok(ValueType::Unestablished)
            }
            _ => Err(error(
                location,
                "concatenation needs two strings or two positional arrays",
            )),
        },
    }
}

fn boolean(value: ConstructedValue, location: &Location) -> Result<bool, QueryError> {
    if let ConstructedValue::Boolean(value) = value {
        Ok(value)
    } else {
        Err(error(location, "condition needs a fixed Boolean"))
    }
}

fn integer(value: ConstructedValue, location: &Location) -> Result<i64, QueryError> {
    if let ConstructedValue::Integer(value) = value {
        Ok(value)
    } else {
        Err(error(location, "operation needs integers"))
    }
}

fn scalar(
    value: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    budget.charge_results(1, location.clone())?;
    Ok(value)
}

fn exact_float(value: i64, location: &Location) -> Result<f64, QueryError> {
    let magnitude = value.unsigned_abs();
    let significant_bits = u64::BITS - magnitude.leading_zeros();
    if significant_bits > 53 && magnitude.trailing_zeros() < significant_bits - 53 {
        return Err(error(
            location,
            "integer cannot be represented exactly as binary64",
        ));
    }
    // The significand test establishes exact representation, including MIN;
    // a float-to-i64 cast would saturate and wrongly accept MAX.
    Ok(value as f64)
}

fn numeric_float(value: ConstructedValue, location: &Location) -> Result<f64, QueryError> {
    match value {
        ConstructedValue::Integer(value) => exact_float(value, location),
        ConstructedValue::Float(value) => {
            finite(value, location)?;
            Ok(value)
        }
        _ => Err(error(location, "operation needs numeric operands")),
    }
}

fn unary(
    operator: ValueUnaryOperator,
    value: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let value = match (operator, value) {
        (ValueUnaryOperator::Not, value) => ConstructedValue::Boolean(!boolean(value, location)?),
        (
            ValueUnaryOperator::Positive,
            value @ (ConstructedValue::Integer(_) | ConstructedValue::Float(_)),
        ) => value,
        (ValueUnaryOperator::Negative, ConstructedValue::Integer(value)) => {
            ConstructedValue::Integer(
                value
                    .checked_neg()
                    .ok_or_else(|| error(location, "integer overflow"))?,
            )
        }
        (ValueUnaryOperator::Negative, ConstructedValue::Float(value)) => {
            ConstructedValue::Float(-value)
        }
        _ => return Err(error(location, "operation needs numeric operands")),
    };
    scalar(value, budget, location)
}

fn binary(
    operator: ValueBinaryOperator,
    left: ConstructedValue,
    right: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    match operator {
        ValueBinaryOperator::Concatenate => match (left, right) {
            (ConstructedValue::String(left), ConstructedValue::String(right)) => {
                let len = sum(left.len(), right.len(), location)?;
                allocation::<u8>(len, location)?;
                budget.charge(len, location.clone())?;
                budget.charge_results(1, location.clone())?;
                let mut value = String::with_capacity(len);
                value.push_str(&left);
                value.push_str(&right);
                Ok(ConstructedValue::String(value))
            }
            (ConstructedValue::Array(left), ConstructedValue::Array(right)) => {
                let len = sum(left.len(), right.len(), location)?;
                let mut values = admit_container(len, budget, location)?;
                values.extend(left);
                values.extend(right);
                Ok(ConstructedValue::Array(values))
            }
            _ => Err(error(
                location,
                "concatenation needs two strings or two positional arrays",
            )),
        },
        ValueBinaryOperator::Equal | ValueBinaryOperator::NotEqual => {
            let equal = equal_values(&left, &right, budget, location)?;
            scalar(
                ConstructedValue::Boolean(if operator == ValueBinaryOperator::Equal {
                    equal
                } else {
                    !equal
                }),
                budget,
                location,
            )
        }
        ValueBinaryOperator::In => {
            let ConstructedValue::Set {
                element_type,
                elements,
            } = right
            else {
                return Err(error(location, "membership needs a supported set"));
            };
            let ty = scalar_set_type(&left)
                .ok_or_else(|| error(location, "membership needs a supported scalar"))?;
            if element_type.is_some_and(|element_type| element_type != ty) {
                return Err(error(location, "membership element type mismatch"));
            }
            let mut found = false;
            for value in &elements {
                if compare_set_values(&left, value, budget, location)? == Ordering::Equal {
                    found = true;
                    break;
                }
            }
            scalar(ConstructedValue::Boolean(found), budget, location)
        }
        ValueBinaryOperator::And | ValueBinaryOperator::Or => {
            let a = boolean(left, location)?;
            let b = boolean(right, location)?;
            scalar(
                ConstructedValue::Boolean(if operator == ValueBinaryOperator::And {
                    a && b
                } else {
                    a || b
                }),
                budget,
                location,
            )
        }
        ValueBinaryOperator::IntegerDivide | ValueBinaryOperator::Modulo => {
            let a = integer(left, location)?;
            let b = integer(right, location)?;
            if b == 0 {
                return Err(error(location, "division by zero"));
            }
            let value = if operator == ValueBinaryOperator::IntegerDivide {
                a.checked_div(b)
            } else {
                a.checked_rem(b)
            }
            .ok_or_else(|| error(location, "integer overflow"))?;
            scalar(ConstructedValue::Integer(value), budget, location)
        }
        ValueBinaryOperator::Add
        | ValueBinaryOperator::Subtract
        | ValueBinaryOperator::Multiply => {
            if let (ConstructedValue::Integer(a), ConstructedValue::Integer(b)) = (&left, &right) {
                let value = match operator {
                    ValueBinaryOperator::Add => a.checked_add(*b),
                    ValueBinaryOperator::Subtract => a.checked_sub(*b),
                    ValueBinaryOperator::Multiply => a.checked_mul(*b),
                    _ => unreachable!("integer arithmetic"),
                }
                .ok_or_else(|| error(location, "integer overflow"))?;
                return scalar(ConstructedValue::Integer(value), budget, location);
            }
            float_operation(
                operator,
                numeric_float(left, location)?,
                numeric_float(right, location)?,
                budget,
                location,
            )
        }
        ValueBinaryOperator::Divide => float_operation(
            operator,
            numeric_float(left, location)?,
            numeric_float(right, location)?,
            budget,
            location,
        ),
        ValueBinaryOperator::Less
        | ValueBinaryOperator::LessEqual
        | ValueBinaryOperator::Greater
        | ValueBinaryOperator::GreaterEqual => {
            let order = if let (ConstructedValue::Integer(a), ConstructedValue::Integer(b)) =
                (&left, &right)
            {
                a.cmp(b)
            } else {
                numeric_float(left, location)?
                    .partial_cmp(&numeric_float(right, location)?)
                    .expect("finite numeric operands")
            };
            let value = match operator {
                ValueBinaryOperator::Less => order == Ordering::Less,
                ValueBinaryOperator::LessEqual => order != Ordering::Greater,
                ValueBinaryOperator::Greater => order == Ordering::Greater,
                ValueBinaryOperator::GreaterEqual => order != Ordering::Less,
                _ => unreachable!("comparison"),
            };
            scalar(ConstructedValue::Boolean(value), budget, location)
        }
    }
}

fn float_operation(
    operator: ValueBinaryOperator,
    a: f64,
    b: f64,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let (value, mathematically_zero) = match operator {
        ValueBinaryOperator::Add => (a + b, a == -b),
        ValueBinaryOperator::Subtract => (a - b, a == b),
        ValueBinaryOperator::Multiply => (a * b, a == 0.0 || b == 0.0),
        ValueBinaryOperator::Divide => {
            if b == 0.0 {
                return Err(error(location, "division by zero"));
            }
            (a / b, a == 0.0)
        }
        _ => unreachable!("floating arithmetic"),
    };
    finite(value, location)?;
    if value == 0.0 && !mathematically_zero {
        return Err(error(location, "nonzero floating result rounded to zero"));
    }
    scalar(ConstructedValue::Float(value), budget, location)
}

fn equal_values(
    left: &ConstructedValue,
    right: &ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match (left, right) {
            (ConstructedValue::Boolean(a), ConstructedValue::Boolean(b)) => a == b,
            (ConstructedValue::Integer(a), ConstructedValue::Integer(b)) => a == b,
            (ConstructedValue::Float(a), ConstructedValue::Float(b)) => a == b,
            (ConstructedValue::String(a), ConstructedValue::String(b)) => {
                budget.charge(a.len().min(b.len()), location.clone())?;
                a == b
            }
            (ConstructedValue::Member(a), ConstructedValue::Member(b)) => {
                check_member_pair(a, b, budget, location)?;
                if a.type_id != b.type_id {
                    return Err(error(location, "equality needs compatible nominal types"));
                }
                a.member_id == b.member_id
            }
            (
                ConstructedValue::KeyedArray {
                    axes: a,
                    entries: a_entries,
                },
                ConstructedValue::KeyedArray {
                    axes: b,
                    entries: b_entries,
                },
            ) => {
                if a.len() != b.len() || a_entries.len() != b_entries.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    budget.charge(1, location.clone())?;
                    if a.key_type != b.key_type {
                        return Err(error(location, "equality needs compatible keyed types"));
                    }
                    if !equal_keys(&a.keys, &b.keys, budget, location)? {
                        return Ok(false);
                    }
                }
                for (a, b) in a_entries.iter().zip(b_entries) {
                    budget.charge(1, location.clone())?;
                    if !equal_keys(&a.key, &b.key, budget, location)?
                        || !equal_values(&a.value, &b.value, budget, location)?
                    {
                        return Ok(false);
                    }
                }
                true
            }
            (ConstructedValue::Array(a), ConstructedValue::Array(b))
            | (ConstructedValue::Tuple(a), ConstructedValue::Tuple(b))
            | (
                ConstructedValue::Set { elements: a, .. },
                ConstructedValue::Set { elements: b, .. },
            ) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    if !equal_values(a, b, budget, location)? {
                        return Ok(false);
                    }
                }
                true
            }
            (ConstructedValue::Record(a), ConstructedValue::Record(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (label, value) in a {
                    let mut other = None;
                    for (other_label, other_value) in b {
                        budget.charge(
                            sum(1, label.len().min(other_label.len()), location)?,
                            location.clone(),
                        )?;
                        if label == other_label {
                            other = Some(other_value);
                            break;
                        }
                    }
                    let Some(other) = other else {
                        return Ok(false);
                    };
                    if !equal_values(value, other, budget, location)? {
                        return Ok(false);
                    }
                }
                true
            }
            _ => return Err(error(location, "equality needs compatible types")),
        })
    })
}

fn run<'a, Q, H: ValueQueryHost<Q>>(
    expression: &'a ValueExpression<Q>,
    queries: &mut H,
    locals: &mut Vec<(&'a str, ConstructedValue)>,
    budget: &mut Budget,
    check_value: &mut impl FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
) -> Result<ConstructedValue, H::Error> {
    let location = Location::Query(expression.range.clone());
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        let value = (|| -> Result<ConstructedValue, H::Error> {
            let location = location.clone();
            match &expression.value {
                ValueExpressionKind::NativeQuery(query) => {
                    queries.evaluate_query(query, locals, budget)
                }
                ValueExpressionKind::One(array) => {
                    let ConstructedValue::Array(values) =
                        run(array, queries, locals, budget, check_value)?
                    else {
                        return Err(error(&location, "one needs a positional array").into());
                    };
                    Ok(take_one(values, expression.range.clone())?)
                }
                ValueExpressionKind::Literal(value) => {
                    Ok(copy_constructed_value(value, budget, location)?)
                }
                ValueExpressionKind::Name(name) => {
                    for (local, value) in locals.iter().rev() {
                        budget.charge(
                            sum(1, name.len().min(local.len()), &location)?,
                            location.clone(),
                        )?;
                        if name == local {
                            return Ok(copy_constructed_value(value, budget, location)?);
                        }
                    }
                    admit_diagnostic("undefined binding: ", name, budget, &location)?;
                    queries.read_binding(name, &expression.range, budget)
                }
                ValueExpressionKind::Array(children)
                | ValueExpressionKind::Tuple(children)
                | ValueExpressionKind::Set(children) => {
                    let mut values = admit_container(children.len(), budget, &location)?;
                    let mut element_type = None;
                    for child in children {
                        let value = run(child, queries, locals, budget, check_value)?;
                        if matches!(expression.value, ValueExpressionKind::Set(_)) {
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
                    Ok(match expression.value {
                        ValueExpressionKind::Array(_) => ConstructedValue::Array(values),
                        ValueExpressionKind::Tuple(_) => ConstructedValue::Tuple(values),
                        ValueExpressionKind::Set(_) => ConstructedValue::Set {
                            element_type,
                            elements: values,
                        },
                        _ => unreachable!("collection expression"),
                    })
                }
                ValueExpressionKind::KeyedArray { axes, entries } => {
                    let axes = match axes {
                        Some(axes) => Some(copy_axes(axes, budget, &location)?),
                        None => None,
                    };
                    let mut values = admit_container(entries.len(), budget, &location)?;
                    for (key, head) in entries {
                        let key_location = Location::Query(key.range.clone());
                        let keys = value_keys(
                            run(key, queries, locals, budget, check_value)?,
                            budget,
                            &key_location,
                        )?;
                        let value = run(head, queries, locals, budget, check_value)?;
                        values.push(Located::new(
                            KeyedEntry { key: keys, value },
                            key.range.clone(),
                        ));
                    }
                    Ok(finish_keyed(axes, values, budget, &location)?)
                }
                ValueExpressionKind::KeyedComprehension {
                    clauses,
                    key,
                    head,
                    empty_axis_sources,
                } => {
                    let mut output = Construction::Keyed {
                        key,
                        entries: admit_container(0, budget, &location)?,
                    };
                    expand(
                        clauses,
                        head,
                        queries,
                        locals,
                        &mut output,
                        budget,
                        &location,
                        check_value,
                    )?;
                    let Construction::Keyed { entries, .. } = output else {
                        unreachable!("keyed construction");
                    };
                    let axes = if entries.is_empty() {
                        let sources = empty_axis_sources
                            .as_ref()
                            .ok_or_else(|| error(&location, "unestablished empty keyed axes"))?;
                        allocation::<KeyAxis>(sources.len(), &location)?;
                        budget.charge(sources.len(), location.clone())?;
                        budget.charge_results(sources.len(), location.clone())?;
                        let mut axes = Vec::with_capacity(sources.len());
                        for source in sources {
                            let value = run(source, queries, locals, budget, check_value)?;
                            axes.push(source_axis(
                                value,
                                budget,
                                &Location::Query(source.range.clone()),
                            )?);
                        }
                        Some(axes)
                    } else {
                        None
                    };
                    Ok(finish_keyed(axes, entries, budget, &location)?)
                }
                ValueExpressionKind::Record(fields) => {
                    let mut values = admit_container(fields.len(), budget, &location)?;
                    for (label, child) in fields {
                        budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
                        let value = run(child, queries, locals, budget, check_value)?;
                        values.push((label.value.to_owned(), value));
                    }
                    Ok(ConstructedValue::Record(values))
                }
                ValueExpressionKind::Unary { operator, operand } => Ok(unary(
                    *operator,
                    run(operand, queries, locals, budget, check_value)?,
                    budget,
                    &location,
                )?),
                ValueExpressionKind::Binary {
                    operator,
                    left,
                    right,
                } => {
                    let left = run(left, queries, locals, budget, check_value)?;
                    if matches!(operator, ValueBinaryOperator::And | ValueBinaryOperator::Or) {
                        let a = match left {
                            ConstructedValue::Boolean(value) => value,
                            _ => {
                                return Err(
                                    error(&location, "condition needs a fixed Boolean").into()
                                );
                            }
                        };
                        if (*operator == ValueBinaryOperator::And && !a)
                            || (*operator == ValueBinaryOperator::Or && a)
                        {
                            return Ok(scalar(ConstructedValue::Boolean(a), budget, &location)?);
                        }
                        let right = run(right, queries, locals, budget, check_value)?;
                        return Ok(binary(
                            *operator,
                            ConstructedValue::Boolean(a),
                            right,
                            budget,
                            &location,
                        )?);
                    }
                    let right = run(right, queries, locals, budget, check_value)?;
                    Ok(binary(*operator, left, right, budget, &location)?)
                }
                ValueExpressionKind::Field { value, field } => {
                    let ConstructedValue::Record(fields) =
                        run(value, queries, locals, budget, check_value)?
                    else {
                        return Err(error(&location, "field access needs a record").into());
                    };
                    for (label, value) in fields {
                        budget.charge(
                            sum(1, label.len().min(field.value.len()), &location)?,
                            location.clone(),
                        )?;
                        if label == field.value {
                            return Ok(value);
                        }
                    }
                    admit_diagnostic("missing record field: ", &field.value, budget, &location)?;
                    Err(error(&location, format!("missing record field: {}", field.value)).into())
                }
                ValueExpressionKind::TupleAccess { value, position } => {
                    let ConstructedValue::Tuple(values) =
                        run(value, queries, locals, budget, check_value)?
                    else {
                        return Err(error(&location, "tuple access needs a tuple").into());
                    };
                    Ok(take_position(values, position.value, budget, &location)?)
                }
                ValueExpressionKind::ArrayAccess { array, index } => {
                    let array = run(array, queries, locals, budget, check_value)?;
                    let index = run(index, queries, locals, budget, check_value)?;
                    match array {
                        ConstructedValue::Array(values) => {
                            let position =
                                integer(index, &Location::Query(expression.range.clone()))?;
                            let position = usize::try_from(position)
                                .map_err(|_| error(&location, "missing array position"))?;
                            Ok(take_position(values, position, budget, &location)?)
                        }
                        ConstructedValue::KeyedArray { axes, entries } => {
                            let key = value_keys(
                                index,
                                budget,
                                &Location::Query(expression.range.clone()),
                            )?;
                            key_positions(&key, &axes, budget, &location)?;
                            for entry in entries {
                                budget.charge(1, location.clone())?;
                                if equal_keys(&key, &entry.key, budget, &location)? {
                                    return Ok(entry.value);
                                }
                            }
                            Err(error(&location, "missing keyed array cell").into())
                        }
                        _ => Err(error(&location, "array access needs an array").into()),
                    }
                }
                ValueExpressionKind::IndexSet(array) => {
                    match run(array, queries, locals, budget, check_value)? {
                        ConstructedValue::Array(values) => {
                            let len = i64::try_from(values.len())
                                .map_err(|_| error(&location, "index overflow"))?;
                            Ok(integer_range(1, len, budget, &location)?)
                        }
                        ConstructedValue::KeyedArray { mut axes, .. } if axes.len() == 1 => {
                            let axis = axes.pop().expect("rank one");
                            let mut values = admit_container(axis.keys.len(), budget, &location)?;
                            let mut element_type = Some(key_set_type(axis.key_type));
                            for key in axis.keys {
                                budget.charge(1, location.clone())?;
                                let value = key_value(key);
                                insert_set(
                                    &mut values,
                                    &mut element_type,
                                    value,
                                    false,
                                    budget,
                                    &location,
                                )?;
                            }
                            Ok(ConstructedValue::Set {
                                element_type,
                                elements: values,
                            })
                        }
                        ConstructedValue::KeyedArray { .. } => {
                            Err(error(&location, "index_set needs rank one").into())
                        }
                        _ => Err(error(&location, "index_set needs an array").into()),
                    }
                }
                ValueExpressionKind::Entries(array) => Ok(entries(
                    run(array, queries, locals, budget, check_value)?,
                    budget,
                    &location,
                )?),
                ValueExpressionKind::ZipExact { left, right } => {
                    let left = run(left, queries, locals, budget, check_value)?;
                    let right = run(right, queries, locals, budget, check_value)?;
                    Ok(zip_exact(left, right, budget, &location)?)
                }
                ValueExpressionKind::Reduce { operator, array } => Ok(reduce(
                    *operator,
                    run(array, queries, locals, budget, check_value)?,
                    budget,
                    &location,
                )?),
                ValueExpressionKind::GroupBy { rows, field } => Ok(group_by(
                    run(rows, queries, locals, budget, check_value)?,
                    field,
                    budget,
                    &location,
                )?),
                ValueExpressionKind::Require {
                    condition,
                    value,
                    message,
                } => {
                    if !boolean(
                        run(condition, queries, locals, budget, check_value)?,
                        &Location::Query(condition.range.clone()),
                    )? {
                        admit_diagnostic("", &message.value, budget, &location)?;
                        return Err(error(&location, message.value.to_owned()).into());
                    }
                    run(value, queries, locals, budget, check_value)
                }
                ValueExpressionKind::Range { lower, upper } => {
                    let lower =
                        integer(run(lower, queries, locals, budget, check_value)?, &location)?;
                    let upper =
                        integer(run(upper, queries, locals, budget, check_value)?, &location)?;
                    Ok(integer_range(lower, upper, budget, &location)?)
                }
                ValueExpressionKind::Conditional {
                    branches,
                    otherwise,
                } => {
                    for (condition, value) in branches {
                        if boolean(
                            run(condition, queries, locals, budget, check_value)?,
                            &Location::Query(condition.range.clone()),
                        )? {
                            return run(value, queries, locals, budget, check_value);
                        }
                    }
                    run(otherwise, queries, locals, budget, check_value)
                }
                ValueExpressionKind::Comprehension {
                    constructor,
                    clauses,
                    head,
                } => {
                    let mut output = match constructor {
                        ValueConstructor::Array => {
                            Construction::Array(admit_container(0, budget, &location)?)
                        }
                        ValueConstructor::Set => Construction::Set {
                            elements: admit_container(0, budget, &location)?,
                            element_type: None,
                        },
                    };
                    expand(
                        clauses,
                        head,
                        queries,
                        locals,
                        &mut output,
                        budget,
                        &location,
                        check_value,
                    )?;
                    Ok(match output {
                        Construction::Array(values) => ConstructedValue::Array(values),
                        Construction::Set {
                            elements,
                            element_type,
                        } => ConstructedValue::Set {
                            element_type,
                            elements,
                        },
                        Construction::Keyed { .. } => unreachable!("positional construction"),
                    })
                }
            }
        })()?;
        check_value(&value, budget, &location)?;
        Ok(value)
    })
}

fn entries(
    array: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let ConstructedValue::KeyedArray { axes, entries } = array else {
        return Err(error(location, "entries needs a keyed array"));
    };
    check_keyed(&axes, &entries, budget, location)?;
    let mut records = admit_container(entries.len(), budget, location)?;
    for entry in entries {
        // The outer array and record add two levels around a moved cell.
        budget.within_evaluation(2, location.clone(), |budget| {
            copy_size(&entry.value, budget, location).map(|_| ())
        })?;
        let key = if entry.key.len() == 1 {
            budget.charge(1, location.clone())?;
            key_value(entry.key.into_iter().next().expect("rank one"))
        } else {
            let mut components = admit_container(entry.key.len(), budget, location)?;
            for key in entry.key {
                budget.charge(1, location.clone())?;
                components.push(key_value(key));
            }
            ConstructedValue::Tuple(components)
        };
        budget.within_evaluation(2, location.clone(), |budget| {
            copy_size(&key, budget, location).map(|_| ())
        })?;
        let mut fields = admit_container(2, budget, location)?;
        budget.charge("key".len() + "value".len(), location.clone())?;
        fields.push(("key".into(), key));
        fields.push(("value".into(), entry.value));
        records.push(ConstructedValue::Record(fields));
    }
    Ok(ConstructedValue::Array(records))
}

fn zip_exact(
    left: ConstructedValue,
    right: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let (
        ConstructedValue::KeyedArray {
            axes,
            entries: left,
        },
        ConstructedValue::KeyedArray {
            axes: right_axes,
            entries: right,
        },
    ) = (left, right)
    else {
        return Err(error(location, "zip_exact needs keyed arrays"));
    };
    check_keyed(&axes, &left, budget, location)?;
    check_keyed(&right_axes, &right, budget, location)?;
    budget.charge(1, location.clone())?;
    if axes.len() != right_axes.len() {
        return Err(error(location, "zip_exact needs identical ordered axes"));
    }
    for (a, b) in axes.iter().zip(&right_axes) {
        budget.charge(1, location.clone())?;
        if a.key_type != b.key_type || !equal_keys(&a.keys, &b.keys, budget, location)? {
            return Err(error(location, "zip_exact needs identical ordered axes"));
        }
    }
    budget.charge(1, location.clone())?;
    if left.len() != right.len() {
        return Err(error(location, "zip_exact needs identical present keys"));
    }
    let mut entries = admit_container(left.len(), budget, location)?;
    for (a, b) in left.into_iter().zip(right) {
        if !equal_keys(&a.key, &b.key, budget, location)? {
            return Err(error(location, "zip_exact needs identical present keys"));
        }
        // The keyed container and tuple add two levels around moved values.
        budget.within_evaluation(2, location.clone(), |budget| {
            copy_size(&a.value, budget, location)?;
            copy_size(&b.value, budget, location).map(|_| ())
        })?;
        let mut pair = admit_container(2, budget, location)?;
        pair.push(a.value);
        pair.push(b.value);
        entries.push(KeyedEntry {
            key: a.key,
            value: ConstructedValue::Tuple(pair),
        });
    }
    Ok(ConstructedValue::KeyedArray { axes, entries })
}

fn group_by(
    rows: ConstructedValue,
    field: &Located<String>,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let ConstructedValue::Array(rows) = rows else {
        return Err(error(location, "group_by needs a positional array"));
    };
    let selector = Location::Query(field.range.clone());
    let mut groups = admit_container(0, budget, location)?;
    let mut key_type = None;
    for row in rows {
        budget.charge(1, location.clone())?;
        let ConstructedValue::Record(fields) = &row else {
            return Err(error(location, "group_by needs record rows"));
        };
        // Moving rows adds a group record and items array around them. Check
        // that added depth without copying or charging their result units again.
        budget.within_evaluation(2, location.clone(), |budget| {
            copy_size(&row, budget, location).map(|_| ())
        })?;
        let mut key = None;
        for (label, value) in fields {
            budget.charge(
                sum(1, label.len().min(field.value.len()), &selector)?,
                selector.clone(),
            )?;
            if label == &field.value {
                key = Some(value);
                break;
            }
        }
        let Some(key) = key else {
            admit_diagnostic("missing record field: ", &field.value, budget, &selector)?;
            return Err(error(
                &selector,
                format!("missing record field: {}", field.value),
            ));
        };
        let ty = match key {
            ConstructedValue::Integer(_) => ValueType::Integer,
            ConstructedValue::String(_) => ValueType::String,
            ConstructedValue::Member(member) => ValueType::Member(member.type_id),
            _ => {
                return Err(error(
                    &selector,
                    "group_by keys need integers, strings or nominal members",
                ));
            }
        };
        budget.charge(1, selector.clone())?;
        if key_type.as_ref().is_some_and(|previous| previous != &ty) {
            return Err(error(&selector, "group_by needs homogeneous key types"));
        }
        key_type = Some(ty);
        let mut matched = None;
        for (index, group) in groups.iter().enumerate() {
            let ConstructedValue::Record(fields) = group else {
                unreachable!("group record")
            };
            if equal_values(key, &fields[0].1, budget, &selector)? {
                matched = Some(index);
                break;
            }
        }
        let index = match matched {
            Some(index) => index,
            None => {
                let key = copy_constructed_value(key, budget, selector.clone())?;
                let mut fields = admit_container(2, budget, location)?;
                budget.charge("key".len() + "items".len(), location.clone())?;
                fields.push(("key".into(), key));
                fields.push((
                    "items".into(),
                    ConstructedValue::Array(admit_container(0, budget, location)?),
                ));
                allocation::<ConstructedValue>(sum(groups.len(), 1, location)?, location)?;
                budget.charge(1, location.clone())?;
                budget.charge_results(1, location.clone())?;
                groups.push(ConstructedValue::Record(fields));
                groups.len() - 1
            }
        };
        let ConstructedValue::Record(fields) = &mut groups[index] else {
            unreachable!("group record")
        };
        let ConstructedValue::Array(items) = &mut fields[1].1 else {
            unreachable!("group items")
        };
        allocation::<ConstructedValue>(sum(items.len(), 1, location)?, location)?;
        budget.charge(1, location.clone())?;
        budget.charge_results(1, location.clone())?;
        items.push(row);
    }
    Ok(ConstructedValue::Array(groups))
}

fn reduce(
    operator: ValueReducer,
    array: ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    // Borrow the materialized values so keyed arrays need no intermediate copy.
    let (positional, keyed) = match &array {
        ConstructedValue::Array(values) => (values.as_slice(), &[][..]),
        ConstructedValue::KeyedArray { entries, .. } => (&[][..], entries.as_slice()),
        _ => return Err(error(location, "reducer needs an array")),
    };
    let len = positional.len().max(keyed.len());
    let values = positional
        .iter()
        .chain(keyed.iter().map(|entry| &entry.value));
    let value = match operator {
        ValueReducer::Length => {
            budget.charge(1, location.clone())?;
            ConstructedValue::Integer(
                i64::try_from(len).map_err(|_| error(location, "length overflow"))?,
            )
        }
        ValueReducer::Sum => {
            let mut total = 0_i64;
            for value in values {
                budget.charge(1, location.clone())?;
                let ConstructedValue::Integer(value) = value else {
                    return Err(error(location, "sum needs integer array elements"));
                };
                total = total
                    .checked_add(*value)
                    .ok_or_else(|| error(location, "integer sum overflow"))?;
            }
            ConstructedValue::Integer(total)
        }
        ValueReducer::Forall | ValueReducer::Exists => {
            let mut result = operator == ValueReducer::Forall;
            for value in values {
                budget.charge(1, location.clone())?;
                let ConstructedValue::Boolean(value) = value else {
                    return Err(error(
                        location,
                        "quantifier needs fixed Boolean array elements",
                    ));
                };
                // Scan every element even once the Boolean result is decisive.
                if operator == ValueReducer::Forall {
                    result &= value;
                } else {
                    result |= value;
                }
            }
            ConstructedValue::Boolean(result)
        }
    };
    scalar(value, budget, location)
}

fn take_position(
    values: Vec<ConstructedValue>,
    position: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let index = position
        .checked_sub(1)
        .filter(|index| *index < values.len())
        .ok_or_else(|| error(location, "missing collection position"))?;
    // Moving an admitted child needs no result readmission. Charge the traversal
    // that consumes preceding positions before selecting it.
    budget.charge(sum(index, 1, location)?, location.clone())?;
    Ok(values.into_iter().nth(index).expect("position checked"))
}

fn integer_range(
    lower: i64,
    upper: i64,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let len = if upper < lower {
        0
    } else {
        usize::try_from(i128::from(upper) - i128::from(lower) + 1)
            .map_err(|_| error(location, "range cardinality overflow"))?
    };
    allocation::<ConstructedValue>(len, location)?;
    budget.charge(len, location.clone())?;
    let units = sum(
        1,
        len.checked_mul(2)
            .ok_or_else(|| error(location, "range allocation overflow"))?,
        location,
    )?;
    budget.charge_results(units, location.clone())?;
    let mut elements = Vec::with_capacity(len);
    for offset in 0..len {
        let value = i64::try_from(i128::from(lower) + offset as i128)
            .map_err(|_| error(location, "range value overflow"))?;
        elements.push(ConstructedValue::Integer(value));
    }
    Ok(ConstructedValue::Set {
        element_type: Some(SetElementType::Integer),
        elements,
    })
}

fn insert_set(
    values: &mut Vec<ConstructedValue>,
    element_type: &mut Option<SetElementType>,
    value: ConstructedValue,
    admit_slot: bool,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    let ty = scalar_set_type(&value)
        .ok_or_else(|| error(location, "set elements must have one supported scalar type"))?;
    if element_type.is_some_and(|previous| previous != ty) {
        return Err(error(location, "set element type mismatch"));
    }
    *element_type = Some(ty);
    if let ConstructedValue::Member(member) = &value {
        for previous in values.iter() {
            if let ConstructedValue::Member(previous) = previous {
                check_member_pair(previous, member, budget, location)?;
            }
        }
    }
    let mut position = values.len();
    for (index, existing) in values.iter().enumerate() {
        match compare_set_values(&value, existing, budget, location)? {
            Ordering::Equal => return Ok(()),
            Ordering::Less => {
                position = index;
                break;
            }
            Ordering::Greater => {}
        }
    }
    allocation::<ConstructedValue>(sum(values.len(), 1, location)?, location)?;
    budget.charge(values.len() - position, location.clone())?;
    if admit_slot {
        budget.charge_results(1, location.clone())?;
    }
    values.insert(position, value);
    Ok(())
}

enum Construction<'a, Q> {
    Array(Vec<ConstructedValue>),
    Set {
        elements: Vec<ConstructedValue>,
        element_type: Option<SetElementType>,
    },
    Keyed {
        key: &'a ValueExpression<Q>,
        entries: Vec<Located<KeyedEntry>>,
    },
}

// This is the runtime clause chain, not a second expression evaluator. Each
// source runs before its own binders; scope restoration covers every Result.
#[expect(
    clippy::too_many_arguments,
    reason = "Clause expansion retains its scope, output and active budget with the same host check"
)]
fn expand<'a, Q, H: ValueQueryHost<Q>>(
    clauses: &'a [Located<ValueGenerator<Q>>],
    head: &'a ValueExpression<Q>,
    queries: &mut H,
    locals: &mut Vec<(&'a str, ConstructedValue)>,
    output: &mut Construction<'a, Q>,
    budget: &mut Budget,
    location: &Location,
    check_value: &mut impl FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
) -> Result<(), H::Error> {
    let Some((clause, rest)) = clauses.split_first() else {
        match output {
            Construction::Array(values) => {
                let value = run(head, queries, locals, budget, check_value)?;
                allocation::<ConstructedValue>(sum(values.len(), 1, location)?, location)?;
                budget.charge_results(1, location.clone())?;
                values.push(value);
            }
            Construction::Set {
                elements,
                element_type,
            } => {
                let value = run(head, queries, locals, budget, check_value)?;
                insert_set(elements, element_type, value, true, budget, location)?;
            }
            Construction::Keyed { key, entries } => {
                let key_location = Location::Query(key.range.clone());
                let keys = value_keys(
                    run(key, queries, locals, budget, check_value)?,
                    budget,
                    &key_location,
                )?;
                let value = run(head, queries, locals, budget, check_value)?;
                allocation::<Located<KeyedEntry>>(
                    sum(entries.len(), 1, &key_location)?,
                    &key_location,
                )?;
                budget.charge(1, key_location.clone())?;
                budget.charge_results(1, key_location)?;
                entries.push(Located::new(
                    KeyedEntry { key: keys, value },
                    key.range.clone(),
                ));
            }
        }
        return Ok(());
    };
    let clause_location = Location::Query(clause.range.clone());
    budget.within_evaluation(
        1,
        clause_location.clone(),
        |budget| -> Result<(), H::Error> {
            budget.charge(1, clause_location.clone())?;
            match &clause.value {
                ValueGenerator::Bind {
                    binder,
                    value,
                    filter,
                } => {
                    budget.charge(1, clause_location.clone())?;
                    let value = run(value, queries, locals, budget, check_value)?;
                    let saved = locals.len();
                    let result = (|| -> Result<(), H::Error> {
                        if let Some(name) = &binder.value {
                            push_metadata(locals, (name, value), budget, &clause_location)?;
                        }
                        if filter_passes(filter, queries, locals, budget, check_value)? {
                            expand(
                                rest,
                                head,
                                queries,
                                locals,
                                output,
                                budget,
                                location,
                                check_value,
                            )?;
                        }
                        Ok(())
                    })();
                    locals.truncate(saved);
                    result
                }
                ValueGenerator::In {
                    binders,
                    source,
                    filter,
                } => {
                    let source_value = run(source, queries, locals, budget, check_value)?;
                    let values = match source_value {
                        ConstructedValue::Array(values)
                        | ConstructedValue::Set {
                            elements: values, ..
                        } => values,
                        ConstructedValue::KeyedArray { entries, .. } => {
                            let mut values = metadata_vec(entries.len(), budget, &clause_location)?;
                            for entry in entries {
                                budget.charge(1, clause_location.clone())?;
                                values.push(entry.value);
                            }
                            values
                        }
                        _ => {
                            return Err(error(
                                &Location::Query(source.range.clone()),
                                "in source needs an array or supported set",
                            )
                            .into());
                        }
                    };
                    let mut candidates = 1usize;
                    for _ in binders {
                        budget.charge(1, clause_location.clone())?;
                        candidates = candidates.checked_mul(values.len()).ok_or_else(|| {
                            error(&clause_location, "grouped generator cardinality overflow")
                        })?;
                    }
                    // The whole grouped product is admitted before any filter.
                    budget.charge(candidates, clause_location.clone())?;
                    if candidates == 0 {
                        return Ok(());
                    }
                    let mut positions = metadata_vec(binders.len(), budget, &clause_location)?;
                    positions.resize(binders.len(), 0usize);
                    for _ in 0..candidates {
                        let saved = locals.len();
                        let result = (|| -> Result<(), H::Error> {
                            for (binder, position) in binders.iter().zip(&positions) {
                                budget.charge(1, Location::Query(binder.range.clone()))?;
                                if let Some(name) = &binder.value {
                                    let value = copy_constructed_value(
                                        &values[*position],
                                        budget,
                                        Location::Query(binder.range.clone()),
                                    )?;
                                    push_metadata(locals, (name, value), budget, &clause_location)?;
                                }
                            }
                            if filter_passes(filter, queries, locals, budget, check_value)? {
                                expand(
                                    rest,
                                    head,
                                    queries,
                                    locals,
                                    output,
                                    budget,
                                    location,
                                    check_value,
                                )?;
                            }
                            Ok(())
                        })();
                        locals.truncate(saved);
                        result?;
                        for position in positions.iter_mut().rev() {
                            budget.charge(1, clause_location.clone())?;
                            *position += 1;
                            if *position < values.len() {
                                break;
                            }
                            *position = 0;
                        }
                    }
                    Ok(())
                }
            }
        },
    )
}

fn filter_passes<'a, Q, H: ValueQueryHost<Q>>(
    filter: &'a Option<ValueExpression<Q>>,
    queries: &mut H,
    locals: &mut Vec<(&'a str, ConstructedValue)>,
    budget: &mut Budget,
    check_value: &mut impl FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
) -> Result<bool, H::Error> {
    match filter {
        Some(filter) => Ok(boolean(
            run(filter, queries, locals, budget, check_value)?,
            &Location::Query(filter.range.clone()),
        )?),
        None => Ok(true),
    }
}

/// Copy inferred facts under the active metadata allowance.
///
/// # Errors
/// Metadata allocation, work or nesting exhaustion retains the supplied location.
pub fn copy_value_types(
    types: &[ValueType],
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<ValueType>, QueryError> {
    copy_types(types, budget, &location)
}

// Shared tuple/record type products preserve every preflighted branch possibility.
pub(crate) fn extend_type_positions(
    positions: &[Vec<ValueType>],
    types: &[ValueType],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<Vec<ValueType>>, QueryError> {
    let count = positions
        .len()
        .checked_mul(types.len())
        .ok_or_else(|| error(location, "type possibilities overflow"))?;
    let mut expanded = metadata_vec(count, budget, location)?;
    for position in positions {
        for ty in types {
            let mut entries = copy_types(position, budget, location)?;
            let ty = copy_type(ty, budget, location)?;
            push_metadata(&mut entries, ty, budget, location)?;
            expanded.push(entries);
        }
    }
    Ok(expanded)
}
