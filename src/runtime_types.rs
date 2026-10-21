use std::ops::Range;

use crate::{
    ArrayType, Budget, ConstructedValue, EvaluationError, Located, Location, NominalTypeId,
    ProgramHost, QueryError, QueryErrorCategory, SetElementType, ValueType, constructed_value_type,
};

/// An opt-in interface contract, distinct from discovered [`ValueType`] facts.
/// Arrays admit positional or keyed storage with homogeneous elements; sequences
/// and keyed declarations restrict that storage. Tuples have fixed positions and
/// records have the declared labels in any stored order, with optional fields
/// allowed to be omitted. Keyed axes
/// remain runtime facts. Sets may contain any constructed values.
/// `Native` uses the adapter's own kind identity, without erasing its values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclaredType<K> {
    /// Any constructed data value; excludes native and computational values.
    Value,
    Optional(Box<Self>),
    Boolean,
    Integer,
    Float,
    String,
    Member(NominalTypeId),
    /// Homogeneous elements in either positional or keyed storage.
    Array(Box<Self>),
    Sequence(Box<Self>),
    Tuple(Vec<Self>),
    Record(Vec<(String, Self)>),
    Dictionary(Box<Self>, Box<Self>),
    Set(SetElementType),
    Keyed(Box<Self>),
    Native(K),
    /// Computational FIFO; its elements retain this whole declaration.
    Queue(Box<Self>),
    /// Computational set of exact host identities for one native kind.
    IdentitySet(K),
}

/// Contracts for one ordinary trusted Rust adapter operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationSignature<K> {
    pub parameters: Vec<DeclaredType<K>>,
    pub result: DeclaredType<K>,
}

/// Optional runtime admission for a [`ProgramHost`]. Existing hosts need not
/// implement this trait or the projection interface.
pub trait RuntimeTypeHost<C>: ProgramHost<C> {
    type Kind;

    /// Borrow a constructed wrapper for shared language checks. Native values
    /// stay native and return `None`.
    fn constructed_value<'v>(&self, _value: &'v Self::Value) -> Option<&'v ConstructedValue> {
        None
    }

    /// Check the actual native representation against the whole declaration,
    /// including containers of native values. Check every member in the current
    /// context, including owner, source coordinates and stale handles.
    /// A stream's kind tag alone does not establish these facts. Charge scans,
    /// comparisons and diagnostic bytes to `budget`; retain native errors.
    fn admit_native(
        &self,
        value: &Self::Value,
        expected: &DeclaredType<Self::Kind>,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), EvaluationError<Self::Error>>;
}

/// Check arity, declarations and actual arguments in written order, invoke the
/// trusted operation once, then check its actual result before returning it.
/// `arguments` can be a borrowed slice or an owned vector; the operation receives
/// that same representation without a core value copy or result charge.
///
/// The operation uses the same context and budget and must admit fresh outputs
/// before allocating them. This is a Rust adapter boundary, not a registry,
/// language callback or effect-policy check.
///
/// # Errors
/// Arity, declaration and result errors use `call_range`; argument errors use
/// their written range. Native errors retain their original type/location.
/// Work and nesting refusals restore active evaluation state, not spending.
pub fn checked_invoke<C, H, A>(
    signature: &OperationSignature<H::Kind>,
    arguments: A,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    call_range: Range<usize>,
    operation: impl FnOnce(A, &mut C, &mut Budget) -> Result<H::Value, EvaluationError<H::Error>>,
) -> Result<H::Value, EvaluationError<H::Error>>
where
    H: RuntimeTypeHost<C>,
    A: AsRef<[Located<H::Value>]>,
{
    let location = Location::Query(call_range);
    budget.within_evaluation(0, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if signature.parameters.len() != arguments.as_ref().len() {
            return Err(refusal("operation argument count mismatch", budget, &location)?.into());
        }
        for parameter in &signature.parameters {
            validate_declaration(parameter, budget, &location)?;
        }
        validate_declaration(&signature.result, budget, &location)?;
        for (argument, expected) in arguments.as_ref().iter().zip(&signature.parameters) {
            admit_host_value(
                &argument.value,
                expected,
                host,
                context,
                budget,
                Location::Query(argument.range.clone()),
            )?;
        }
        let result = operation(arguments, context, budget)?;
        admit_host_value(&result, &signature.result, host, context, budget, location)?;
        Ok(result)
    })
}

/// Admit a borrowed native or constructed host value against a declaration.
/// No native `Clone`, `Send`, `Sync` or `'static` bound is required.
///
/// # Errors
/// Language mismatches and malformed declarations use `location`; native
/// admission preserves the adapter's error and source coordinates.
pub fn admit_value<C, H: RuntimeTypeHost<C>>(
    value: &H::Value,
    expected: &DeclaredType<H::Kind>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<(), EvaluationError<H::Error>> {
    validate_declaration(expected, budget, &location)?;
    admit_host_value(value, expected, host, context, budget, location)
}

/// Check actual constructed data, including all nested elements and the current
/// finite-float, label, nominal, set and keyed-shape invariants. Caller-supplied
/// type summaries are never admission evidence. Typed empty sets retain their
/// element identity; an untyped empty set does not establish that identity.
///
/// Successful admission remains valid for this immutable value and declaration.
/// It does not admit future values or replace producer allocation charges,
/// native ownership checks, nominal resolution or data-dependent access checks.
/// This function stores no admission facts or execution cache.
///
/// # Errors
/// Malformed declarations/data, incompatible types and allowance exhaustion
/// retain `location`. Mismatch messages identify nested fields and positions.
pub fn admit_constructed<K>(
    value: &ConstructedValue,
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: Location,
) -> Result<(), QueryError> {
    validate_declaration(expected, budget, &location)?;
    admit_constructed_value(value, expected, budget, location)
}

/// Admit an actual constructed value and retain its declaration through ordinary
/// bindings, access, copying and evaluation. Native declarations belong to hosts.
///
/// ```
/// use zirql::{Budget, ConstructedValue as V, DeclaredType as D, Location,
///             ProgramLimits, declare_constructed, copy_constructed_value};
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 10_000, results: 1_000, nesting: 64, stream_items: 100,
/// });
/// let row = declare_constructed(
///     V::Record(vec![("id".into(), V::Integer(4))]),
///     D::Record(vec![
///         ("id".into(), D::Integer),
///         ("caption".into(), D::Optional(Box::new(D::String))),
///     ]),
///     &mut budget, Location::Query(0..20),
/// )?;
/// let copy = copy_constructed_value(&row, &mut budget, Location::Query(21..24))?;
/// assert_eq!(row, copy);
/// assert!(matches!(row.data(), V::Record(fields) if fields.len() == 1));
/// # Ok::<(), zirql::QueryError>(())
/// ```
///
/// # Errors
/// Returns the same located admission failures as [`admit_constructed`].
/// `Native(())` is not a constructed declaration and is refused at any depth.
pub fn declare_constructed(
    value: ConstructedValue,
    declaration: DeclaredType<()>,
    budget: &mut Budget,
    location: Location,
) -> Result<ConstructedValue, QueryError> {
    admit_constructed(&value, &declaration, budget, location.clone())?;
    reject_native(&declaration, budget, &location)?;
    annotate(value, declaration, budget, &location)
}

pub(crate) fn annotate(
    value: ConstructedValue,
    declaration: DeclaredType<()>,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let mut value = value;
    while let ConstructedValue::Declared {
        value: inner,
        declaration: previous,
    } = value
    {
        if !compatible_declarations(&previous, &declaration, budget, location)? {
            return Err(refusal(
                "incompatible declared underlying types",
                budget,
                location,
            )?);
        }
        value = *inner;
    }
    budget.charge(1, location.clone())?;
    budget.charge_results(1, location.clone())?;
    Ok(ConstructedValue::Declared {
        value: Box::new(value),
        declaration,
    })
}

fn reject_native(
    declaration: &DeclaredType<()>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match declaration {
            DeclaredType::Native(()) | DeclaredType::Queue(_) | DeclaredType::IdentitySet(()) => {
                return Err(refusal(
                    "native declaration requires a host",
                    budget,
                    location,
                )?);
            }
            DeclaredType::Optional(child)
            | DeclaredType::Array(child)
            | DeclaredType::Sequence(child)
            | DeclaredType::Keyed(child) => reject_native(child, budget, location)?,
            DeclaredType::Dictionary(key, value) => {
                reject_native(key, budget, location)?;
                reject_native(value, budget, location)?;
            }
            DeclaredType::Tuple(children) => {
                for child in children {
                    reject_native(child, budget, location)?;
                }
            }
            DeclaredType::Record(fields) => {
                for (_, child) in fields {
                    reject_native(child, budget, location)?;
                }
            }
            _ => {}
        }
        Ok(())
    })
}

/// Copy a declaration without inspecting its values or changing native kinds.
/// Work, child slots, label bytes and nesting are checked before allocation.
/// The returned root is stored inline; hosts charge their own Rc/Arc wrapper.
/// Native kind cloning remains the host's responsibility through `K::clone`.
///
/// ```
/// use zirql::{Budget, DeclaredType, Location, ProgramLimits, copy_declaration};
/// let declaration = DeclaredType::<()>::Sequence(Box::new(DeclaredType::String));
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 100, results: 10, nesting: 8, stream_items: 10,
/// });
/// let copied = copy_declaration(&declaration, &mut budget, &Location::Query(0..4))?;
/// assert_eq!(copied, declaration); // The element contract survives an empty sequence.
/// # Ok::<(), zirql::QueryError>(())
/// ```
///
/// # Errors
/// Refuses exhausted work, result or nesting limits at `location`.
pub fn copy_declaration<K: Clone>(
    declaration: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<DeclaredType<K>, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        let slots = match declaration {
            DeclaredType::Optional(_)
            | DeclaredType::Array(_)
            | DeclaredType::Sequence(_)
            | DeclaredType::Queue(_)
            | DeclaredType::Keyed(_) => 1,
            DeclaredType::Dictionary(_, _) => 2,
            DeclaredType::Tuple(children) => children.len(),
            DeclaredType::Record(fields) => fields.len(),
            _ => 0,
        };
        budget.charge_results(slots, location.clone())?;
        Ok(match declaration {
            DeclaredType::Optional(child) => {
                DeclaredType::Optional(Box::new(copy_declaration(child, budget, location)?))
            }
            DeclaredType::Array(child) => {
                DeclaredType::Array(Box::new(copy_declaration(child, budget, location)?))
            }
            DeclaredType::Sequence(child) => {
                DeclaredType::Sequence(Box::new(copy_declaration(child, budget, location)?))
            }
            DeclaredType::Queue(child) => {
                DeclaredType::Queue(Box::new(copy_declaration(child, budget, location)?))
            }
            DeclaredType::Keyed(child) => {
                DeclaredType::Keyed(Box::new(copy_declaration(child, budget, location)?))
            }
            DeclaredType::Dictionary(key, value) => DeclaredType::Dictionary(
                Box::new(copy_declaration(key, budget, location)?),
                Box::new(copy_declaration(value, budget, location)?),
            ),
            DeclaredType::Tuple(children) => {
                budget.charge(children.len(), location.clone())?;
                let mut copied = Vec::with_capacity(children.len());
                for child in children {
                    copied.push(copy_declaration(child, budget, location)?);
                }
                DeclaredType::Tuple(copied)
            }
            DeclaredType::Record(fields) => {
                budget.charge(fields.len(), location.clone())?;
                let mut copied = Vec::with_capacity(fields.len());
                for (label, child) in fields {
                    budget.charge(label.len(), location.clone())?;
                    budget.charge_results(label.len(), location.clone())?;
                    copied.push((label.clone(), copy_declaration(child, budget, location)?));
                }
                DeclaredType::Record(copied)
            }
            other => other.clone(),
        })
    })
}

pub(crate) fn compatible_declarations<K>(
    left: &DeclaredType<()>,
    right: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    compatible_declarations_with(left, right, budget, location, &|_, _| false)
}

pub(crate) fn compatible_state<K: PartialEq>(
    left: &DeclaredType<K>,
    right: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    compatible_declarations_with(left, right, budget, location, &|a, b| a == b)
}

fn compatible_declarations_with<A, B>(
    left: &DeclaredType<A>,
    right: &DeclaredType<B>,
    budget: &mut Budget,
    location: &Location,
    native: &impl Fn(&A, &B) -> bool,
) -> Result<bool, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match (left, right) {
            (DeclaredType::Optional(left), right) => {
                compatible_declarations_with(left, right, budget, location, native)?
            }
            (left, DeclaredType::Optional(right)) => {
                compatible_declarations_with(left, right, budget, location, native)?
            }
            (DeclaredType::Value, other) => !contains_native(other, budget, location)?,
            (other, DeclaredType::Value) => !contains_native(other, budget, location)?,
            (DeclaredType::Boolean, DeclaredType::Boolean)
            | (DeclaredType::Integer, DeclaredType::Integer)
            | (DeclaredType::Float, DeclaredType::Float)
            | (DeclaredType::String, DeclaredType::String) => true,
            (DeclaredType::Native(a), DeclaredType::Native(b))
            | (DeclaredType::IdentitySet(a), DeclaredType::IdentitySet(b)) => native(a, b),
            (DeclaredType::Member(a), DeclaredType::Member(b)) => a == b,
            (DeclaredType::Set(a), DeclaredType::Set(b)) => {
                a == b || *a == SetElementType::Value || *b == SetElementType::Value
            }
            (DeclaredType::Dictionary(ak, av), DeclaredType::Dictionary(bk, bv)) => {
                compatible_declarations_with(ak, bk, budget, location, native)?
                    && compatible_declarations_with(av, bv, budget, location, native)?
            }
            (DeclaredType::Record(fields), DeclaredType::Dictionary(key, value)) => {
                if !fields.is_empty()
                    && !compatible_declarations_with(
                        &DeclaredType::<A>::String,
                        key,
                        budget,
                        location,
                        native,
                    )?
                {
                    return Ok(false);
                }
                for (_, field) in fields {
                    if !compatible_declarations_with(field, value, budget, location, native)? {
                        return Ok(false);
                    }
                }
                true
            }
            (DeclaredType::Dictionary(key, value), DeclaredType::Record(fields)) => {
                if !fields.is_empty()
                    && !compatible_declarations_with(
                        key,
                        &DeclaredType::<B>::String,
                        budget,
                        location,
                        native,
                    )?
                {
                    return Ok(false);
                }
                for (_, field) in fields {
                    if !compatible_declarations_with(value, field, budget, location, native)? {
                        return Ok(false);
                    }
                }
                true
            }
            (DeclaredType::Array(a), DeclaredType::Array(b))
            | (DeclaredType::Array(a), DeclaredType::Sequence(b) | DeclaredType::Keyed(b))
            | (DeclaredType::Sequence(a) | DeclaredType::Keyed(a), DeclaredType::Array(b))
            | (DeclaredType::Sequence(a), DeclaredType::Sequence(b))
            | (DeclaredType::Keyed(a), DeclaredType::Keyed(b))
            | (DeclaredType::Queue(a), DeclaredType::Queue(b)) => {
                compatible_declarations_with(a, b, budget, location, native)?
            }
            (DeclaredType::Tuple(a), DeclaredType::Tuple(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    if !compatible_declarations_with(a, b, budget, location, native)? {
                        return Ok(false);
                    }
                }
                true
            }
            (DeclaredType::Record(a), DeclaredType::Record(b)) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (label, a) in a {
                    let mut found = None;
                    for (other, b) in b {
                        budget.charge(1 + label.len().min(other.len()), location.clone())?;
                        if label == other {
                            found = Some(b);
                            break;
                        }
                    }
                    let Some(b) = found else {
                        return Ok(false);
                    };
                    if !compatible_declarations_with(a, b, budget, location, native)? {
                        return Ok(false);
                    }
                }
                true
            }
            _ => false,
        })
    })
}

fn admit_host_value<C, H: RuntimeTypeHost<C>>(
    value: &H::Value,
    expected: &DeclaredType<H::Kind>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<(), EvaluationError<H::Error>> {
    budget.within_evaluation(0, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let Some(value) = host.constructed_value(value) {
            return Ok(admit_constructed_value(value, expected, budget, location)?);
        }
        let mut underlying = expected;
        while let DeclaredType::Optional(inner) = underlying {
            budget.charge(1, location.clone())?;
            underlying = inner;
        }
        if matches!(underlying, DeclaredType::Value) {
            return Err(refusal("expected constructed value", budget, &location)?.into());
        }
        host.admit_native(value, expected, context, budget, location)
    })
}

fn admit_constructed_value<K>(
    value: &ConstructedValue,
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(0, location.clone(), |budget| {
        if let ConstructedValue::Declared { value, declaration } = value {
            reject_native(declaration, budget, &location)?;
            admit_constructed_value(value, declaration, budget, location.clone())?;
            if !compatible_declarations(declaration, expected, budget, &location)? {
                return Err(refusal(
                    "incompatible declared underlying types",
                    budget,
                    &location,
                )?);
            }
            return admit_constructed_value(value, expected, budget, location);
        }
        budget.charge(1, location.clone())?;
        match (value, expected) {
            (ConstructedValue::Absent, DeclaredType::Optional(_)) => Ok(()),
            (_, DeclaredType::Optional(element)) => {
                admit_constructed_value(value, element, budget, location)
            }
            (
                ConstructedValue::Array(values),
                DeclaredType::Array(element) | DeclaredType::Sequence(element),
            ) => {
                let label = if matches!(expected, DeclaredType::Sequence(_)) {
                    "sequence element"
                } else {
                    "array element"
                };
                for (index, value) in values.iter().enumerate() {
                    admit_constructed_value(value, element, budget, location.clone())
                        .map_err(|error| nested_error(error, label, Some(index), budget))?;
                }
                Ok(())
            }
            (ConstructedValue::Record(actual), DeclaredType::Record(expected)) => {
                for (index, (label, _)) in actual.iter().enumerate() {
                    for (previous, _) in &actual[..index] {
                        budget.charge(1 + label.len().min(previous.len()), location.clone())?;
                        if label == previous {
                            return Err(refusal("duplicate record label", budget, &location)?);
                        }
                    }
                    let mut declared = false;
                    for (candidate, _) in expected {
                        budget.charge(1 + label.len().min(candidate.len()), location.clone())?;
                        declared |= label == candidate;
                    }
                    if !declared {
                        return Err(refusal(
                            "record label outside declared schema",
                            budget,
                            &location,
                        )?);
                    }
                }
                for (label, expected) in expected {
                    let mut found = None;
                    for (candidate, actual) in actual {
                        budget.charge(1 + label.len().min(candidate.len()), location.clone())?;
                        if label == candidate {
                            found = Some(actual);
                            break;
                        }
                    }
                    let Some(actual) = found else {
                        if matches!(expected, DeclaredType::Optional(_)) {
                            continue;
                        }
                        return Err(nested_error(
                            refusal("missing declared record field", budget, &location)?,
                            label,
                            None,
                            budget,
                        ));
                    };
                    admit_constructed_value(actual, expected, budget, location.clone())
                        .map_err(|error| nested_error(error, label, None, budget))?;
                }
                Ok(())
            }
            _ => {
                // Other shapes use actual discovery, including nominal/keyed/set invariants.
                let actual = constructed_value_type(value, budget, location.clone())?;
                check_type(&actual, expected, budget, &location)
            }
        }
    })
}

pub(crate) fn validate_declaration<K>(
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match expected {
            DeclaredType::Optional(element)
            | DeclaredType::Array(element)
            | DeclaredType::Sequence(element)
            | DeclaredType::Keyed(element)
            | DeclaredType::Queue(element) => {
                validate_declaration(element, budget, location)?;
            }
            DeclaredType::Dictionary(key, value) => {
                validate_declaration(key, budget, location)?;
                validate_declaration(value, budget, location)?;
            }
            DeclaredType::Tuple(elements) => {
                for element in elements {
                    validate_declaration(element, budget, location)?;
                }
            }
            DeclaredType::Record(fields) => {
                for (index, (label, _)) in fields.iter().enumerate() {
                    for (previous, _) in &fields[..index] {
                        budget.charge(1 + label.len().min(previous.len()), location.clone())?;
                        if label == previous {
                            return Err(refusal("duplicate record label", budget, location)?);
                        }
                    }
                }
                for (label, field) in fields {
                    budget.charge(label.len(), location.clone())?;
                    validate_declaration(field, budget, location)?;
                }
            }
            DeclaredType::Value
            | DeclaredType::Boolean
            | DeclaredType::Integer
            | DeclaredType::Float
            | DeclaredType::String
            | DeclaredType::Member(_)
            | DeclaredType::Set(_)
            | DeclaredType::Native(_)
            | DeclaredType::IdentitySet(_) => {}
        }
        Ok(())
    })
}

/// A primitive constructed scalar contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    Boolean,
    Integer,
    Float,
    String,
}

/// Whether incomplete type facts may defer admission until runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeFactPolicy {
    /// Require established facts, including the actual facts in declared wrappers.
    Strict,
    /// Permit `Unestablished` and `ScalarKind`; actual values still need admission.
    Deferred,
}

/// Check retained type facts against an existing declaration.
/// Declared wrappers and nested collection facts use runtime admission's rules.
/// Native kinds and actual values still require host admission.
///
/// # Errors
/// Returns a located declaration mismatch or budget refusal. Deferred facts may
/// remain unestablished; they do not establish a native owner's validity.
pub fn admit_type_facts<K>(
    actual: &ValueType,
    expected: &DeclaredType<K>,
    policy: TypeFactPolicy,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    check_type_facts(
        actual,
        expected,
        policy == TypeFactPolicy::Deferred,
        budget,
        location,
    )
}

/// Check scalar facts with the same rules as runtime declaration admission.
/// Declared wrappers must have a compatible scalar declaration and compatible
/// actual facts. Hosts choose whether their context permits deferred facts.
///
/// # Errors
/// Returns a located type mismatch or budget refusal. This checks facts only;
/// it neither reads a value nor admits native values or domain-specific kinds.
pub fn admit_scalar_type(
    actual: &ValueType,
    expected: ScalarType,
    policy: TypeFactPolicy,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    let expected = match expected {
        ScalarType::Boolean => DeclaredType::<()>::Boolean,
        ScalarType::Integer => DeclaredType::Integer,
        ScalarType::Float => DeclaredType::Float,
        ScalarType::String => DeclaredType::String,
    };
    admit_type_facts(actual, &expected, policy, budget, location)
}

/// Extract a nonnegative integer that fits the platform's `usize`.
/// Hosts supply the diagnostic message for their argument context.
///
/// # Errors
/// Negative or overflowing values return `message` at `location`.
pub fn nonnegative_integer_to_usize(
    value: i64,
    location: Location,
    message: &str,
) -> Result<usize, QueryError> {
    usize::try_from(value).map_err(|_| QueryError {
        location,
        message: message.to_owned(),
        category: QueryErrorCategory::Other,
    })
}

fn check_type<K>(
    actual: &ValueType,
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    check_type_facts(actual, expected, false, budget, location)
}

pub(crate) fn check_known_type(
    actual: &ValueType,
    expected: &DeclaredType<()>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    check_type_facts(actual, expected, true, budget, location)
}

pub(crate) fn check_type_facts<K>(
    actual: &ValueType,
    expected: &DeclaredType<K>,
    deferred: bool,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match (actual, expected) {
            (ValueType::Unestablished, _) if deferred => Ok(()),
            (
                ValueType::ScalarKind,
                DeclaredType::Boolean
                | DeclaredType::Integer
                | DeclaredType::Float
                | DeclaredType::String,
            ) if deferred => Ok(()),
            (ValueType::RecordKind, DeclaredType::Record(_))
            | (ValueType::TupleKind, DeclaredType::Tuple(_))
            | (ValueType::Set(None), DeclaredType::Set(_))
                if deferred =>
            {
                Ok(())
            }
            (
                ValueType::ArrayKind(elements),
                DeclaredType::Array(element)
                | DeclaredType::Sequence(element)
                | DeclaredType::Keyed(element),
            ) if deferred => check_elements(
                elements,
                element,
                "array element",
                deferred,
                budget,
                location,
            ),
            (ValueType::Array(ArrayType::Elements(elements)), DeclaredType::Sequence(element))
                if deferred =>
            {
                check_elements(
                    elements,
                    element,
                    "sequence element",
                    deferred,
                    budget,
                    location,
                )
            }
            (
                ValueType::Declared {
                    actual,
                    declaration,
                },
                expected,
            ) => {
                if !compatible_declarations(declaration, expected, budget, location)? {
                    return Err(refusal(
                        "incompatible declared underlying types",
                        budget,
                        location,
                    )?);
                }
                check_type_facts(actual, expected, deferred, budget, location)
            }
            (_, DeclaredType::Value) => Ok(()),
            (ValueType::Absent, DeclaredType::Optional(_)) => Ok(()),
            (actual, DeclaredType::Optional(element)) => {
                check_type_facts(actual, element, deferred, budget, location)
            }
            (ValueType::Boolean, DeclaredType::Boolean)
            | (ValueType::Integer, DeclaredType::Integer)
            | (ValueType::Float, DeclaredType::Float)
            | (ValueType::String, DeclaredType::String) => Ok(()),
            (ValueType::Member(actual), DeclaredType::Member(expected)) if actual == expected => {
                Ok(())
            }
            (ValueType::Set(actual), DeclaredType::Set(expected))
                if *expected == SetElementType::Value || *actual == Some(*expected) =>
            {
                Ok(())
            }
            (
                ValueType::Array(ArrayType::Positions(elements) | ArrayType::Elements(elements)),
                DeclaredType::Array(element),
            ) => check_elements(
                elements,
                element,
                "array element",
                deferred,
                budget,
                location,
            ),
            (ValueType::Array(ArrayType::Positions(elements)), DeclaredType::Sequence(element)) => {
                check_elements(
                    elements,
                    element,
                    "sequence element",
                    deferred,
                    budget,
                    location,
                )
            }
            (
                ValueType::KeyedArray(actual),
                DeclaredType::Array(element) | DeclaredType::Keyed(element),
            ) => check_elements(
                &actual.elements,
                element,
                "keyed cell",
                deferred,
                budget,
                location,
            ),
            (ValueType::Dictionary { keys, values }, DeclaredType::Dictionary(key, value)) => {
                check_elements(keys, key, "dictionary key", deferred, budget, location)?;
                check_elements(
                    values,
                    value,
                    "dictionary value",
                    deferred,
                    budget,
                    location,
                )
            }
            (ValueType::Record(fields), DeclaredType::Dictionary(key, value)) => {
                for (label, actual) in fields {
                    check_type_facts(&ValueType::String, key, deferred, budget, location)?;
                    check_type_facts(actual, value, deferred, budget, location)
                        .map_err(|error| nested_error(error, label, None, budget))?;
                }
                Ok(())
            }
            (ValueType::RecordKind, DeclaredType::Dictionary(key, _)) if deferred => {
                check_type_facts(&ValueType::String, key, deferred, budget, location)
            }
            (ValueType::Tuple(actual), DeclaredType::Tuple(expected)) => {
                if actual.len() != expected.len() {
                    return Err(refusal("tuple length mismatch", budget, location)?);
                }
                for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    check_type_facts(actual, expected, deferred, budget, location).map_err(
                        |error| nested_error(error, "tuple position", Some(index), budget),
                    )?;
                }
                Ok(())
            }
            (ValueType::Record(actual), DeclaredType::Record(expected)) => {
                for (label, _) in actual {
                    let mut declared = false;
                    for (candidate, _) in expected {
                        budget.charge(1 + label.len().min(candidate.len()), location.clone())?;
                        declared |= label == candidate;
                    }
                    if !declared {
                        return Err(refusal(
                            "record label outside declared schema",
                            budget,
                            location,
                        )?);
                    }
                }
                for (label, expected) in expected {
                    let mut found = None;
                    for (candidate, actual) in actual {
                        budget.charge(1 + label.len().min(candidate.len()), location.clone())?;
                        if label == candidate {
                            found = Some(actual);
                            break;
                        }
                    }
                    let Some(actual) = found else {
                        if matches!(expected, DeclaredType::Optional(_)) {
                            continue;
                        }
                        return Err(nested_error(
                            refusal("missing declared record field", budget, location)?,
                            label,
                            None,
                            budget,
                        ));
                    };
                    check_type_facts(actual, expected, deferred, budget, location)
                        .map_err(|error| nested_error(error, label, None, budget))?;
                }
                Ok(())
            }
            _ => Err(refusal(expected.description(), budget, location)?),
        }
    })
}

fn check_elements<K>(
    elements: &[ValueType],
    expected: &DeclaredType<K>,
    label: &str,
    deferred: bool,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    for (index, actual) in elements.iter().enumerate() {
        check_type_facts(actual, expected, deferred, budget, location)
            .map_err(|error| nested_error(error, label, Some(index), budget))?;
    }
    Ok(())
}

impl<K> DeclaredType<K> {
    fn description(&self) -> &'static str {
        match self {
            Self::Value => "expected constructed value",
            Self::Optional(_) => "expected optional value",
            Self::Boolean => "expected Boolean",
            Self::Integer => "expected integer",
            Self::Float => "expected finite float",
            Self::String => "expected string",
            Self::Member(_) => "expected member of declared nominal type",
            Self::Array(_) => "expected array",
            Self::Sequence(_) => "expected positional sequence",
            Self::Tuple(_) => "expected tuple",
            Self::Record(_) => "expected record",
            Self::Dictionary(_, _) => "expected dictionary",
            Self::Set(_) => "expected set of declared element type",
            Self::Keyed(_) => "expected keyed array",
            Self::Native(_) => "expected native value",
            Self::Queue(_) => "expected computational queue",
            Self::IdentitySet(_) => "expected native identity set",
        }
    }
}

fn refusal(
    message: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<QueryError, QueryError> {
    budget.charge(message.len(), location.clone())?;
    Ok(QueryError {
        location: location.clone(),
        message: message.to_owned(),
        category: QueryErrorCategory::Other,
    })
}

fn nested_error(
    mut error: QueryError,
    label: &str,
    index: Option<usize>,
    budget: &mut Budget,
) -> QueryError {
    // Preserve allowance failures without allocating a growing diagnostic path.
    if error.category != QueryErrorCategory::Other {
        return error;
    }
    let bytes = error
        .message
        .len()
        .saturating_add(label.len())
        .saturating_add(24);
    if let Err(limit) = budget.charge(bytes, error.location.clone()) {
        return limit;
    }
    error.message = match index {
        Some(index) => format!("{label} {}: {}", index + 1, error.message),
        None => format!("field {label}: {}", error.message),
    };
    error
}

// Exact return obligations, unlike the intentionally weaker admission compatibility.
pub(crate) fn same_declaration<K: PartialEq>(
    a: &DeclaredType<K>,
    b: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match (a, b) {
            (DeclaredType::Optional(a), DeclaredType::Optional(b))
            | (DeclaredType::Array(a), DeclaredType::Array(b))
            | (DeclaredType::Sequence(a), DeclaredType::Sequence(b))
            | (DeclaredType::Keyed(a), DeclaredType::Keyed(b))
            | (DeclaredType::Queue(a), DeclaredType::Queue(b)) => {
                same_declaration(a, b, budget, location)?
            }
            (DeclaredType::Dictionary(ak, av), DeclaredType::Dictionary(bk, bv)) => {
                same_declaration(ak, bk, budget, location)?
                    && same_declaration(av, bv, budget, location)?
            }
            (DeclaredType::Tuple(a), DeclaredType::Tuple(b)) if a.len() == b.len() => {
                for (a, b) in a.iter().zip(b) {
                    if !same_declaration(a, b, budget, location)? {
                        return Ok(false);
                    }
                }
                true
            }
            (DeclaredType::Record(a), DeclaredType::Record(b)) if a.len() == b.len() => {
                for (label, a) in a {
                    let mut found = None;
                    for (name, b) in b {
                        budget.charge(1 + label.len().min(name.len()), location.clone())?;
                        if label == name {
                            found = Some(b);
                            break;
                        }
                    }
                    if let Some(b) = found {
                        if !same_declaration(a, b, budget, location)? {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }
                true
            }
            (DeclaredType::Native(a), DeclaredType::Native(b))
            | (DeclaredType::IdentitySet(a), DeclaredType::IdentitySet(b)) => a == b,
            (DeclaredType::Member(a), DeclaredType::Member(b)) => a == b,
            (DeclaredType::Set(a), DeclaredType::Set(b)) => a == b,
            (DeclaredType::Value, DeclaredType::Value)
            | (DeclaredType::Boolean, DeclaredType::Boolean)
            | (DeclaredType::Integer, DeclaredType::Integer)
            | (DeclaredType::Float, DeclaredType::Float)
            | (DeclaredType::String, DeclaredType::String) => true,
            _ => false,
        })
    })
}
pub(crate) fn constructed_declaration<K>(
    declaration: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<DeclaredType<()>, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match declaration {
            DeclaredType::Optional(c) => {
                DeclaredType::Optional(Box::new(constructed_declaration(c, budget, location)?))
            }
            DeclaredType::Array(c) => {
                DeclaredType::Array(Box::new(constructed_declaration(c, budget, location)?))
            }
            DeclaredType::Sequence(c) => {
                DeclaredType::Sequence(Box::new(constructed_declaration(c, budget, location)?))
            }
            DeclaredType::Keyed(c) => {
                DeclaredType::Keyed(Box::new(constructed_declaration(c, budget, location)?))
            }
            DeclaredType::Dictionary(key, value) => DeclaredType::Dictionary(
                Box::new(constructed_declaration(key, budget, location)?),
                Box::new(constructed_declaration(value, budget, location)?),
            ),
            DeclaredType::Tuple(c) => {
                budget.charge(c.len(), location.clone())?;
                DeclaredType::Tuple(
                    c.iter()
                        .map(|c| constructed_declaration(c, budget, location))
                        .collect::<Result<_, _>>()?,
                )
            }
            DeclaredType::Record(c) => {
                budget.charge(c.len(), location.clone())?;
                DeclaredType::Record(
                    c.iter()
                        .map(|(s, c)| {
                            budget.charge(s.len(), location.clone())?;
                            Ok((s.clone(), constructed_declaration(c, budget, location)?))
                        })
                        .collect::<Result<_, QueryError>>()?,
                )
            }
            DeclaredType::Value => DeclaredType::Value,
            DeclaredType::Boolean => DeclaredType::Boolean,
            DeclaredType::Integer => DeclaredType::Integer,
            DeclaredType::Float => DeclaredType::Float,
            DeclaredType::String => DeclaredType::String,
            DeclaredType::Member(id) => DeclaredType::Member(*id),
            DeclaredType::Set(t) => DeclaredType::Set(*t),
            DeclaredType::Native(_) | DeclaredType::Queue(_) | DeclaredType::IdentitySet(_) => {
                return Err(QueryError::query(
                    match location {
                        Location::Query(r) | Location::Source(r) => r.clone(),
                    },
                    "native declaration requires a native value",
                ));
            }
        })
    })
}

/// Whether a declaration contains native or computational values.
/// This inspects Zirql structure only; it does not classify a host's native kinds.
///
/// # Errors
/// Refuses exhausted work or nesting limits at `location`.
pub fn contains_native<K>(
    declaration: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match declaration {
            DeclaredType::Native(_) | DeclaredType::Queue(_) | DeclaredType::IdentitySet(_) => true,
            DeclaredType::Optional(child)
            | DeclaredType::Array(child)
            | DeclaredType::Sequence(child)
            | DeclaredType::Keyed(child) => contains_native(child, budget, location)?,
            DeclaredType::Dictionary(key, value) => {
                contains_native(key, budget, location)? || contains_native(value, budget, location)?
            }
            DeclaredType::Tuple(children) => {
                for child in children {
                    if contains_native(child, budget, location)? {
                        return Ok(true);
                    }
                }
                false
            }
            DeclaredType::Record(children) => {
                for (_, child) in children {
                    if contains_native(child, budget, location)? {
                        return Ok(true);
                    }
                }
                false
            }
            _ => false,
        })
    })
}

/// Whether a contract contains a computational value at any position.
pub(crate) fn contains_computational<K>(
    declaration: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match declaration {
            DeclaredType::Queue(_) | DeclaredType::IdentitySet(_) => true,
            DeclaredType::Optional(c)
            | DeclaredType::Array(c)
            | DeclaredType::Sequence(c)
            | DeclaredType::Keyed(c) => contains_computational(c, budget, location)?,
            DeclaredType::Dictionary(key, value) => {
                contains_computational(key, budget, location)?
                    || contains_computational(value, budget, location)?
            }
            DeclaredType::Tuple(c) => {
                for c in c {
                    if contains_computational(c, budget, location)? {
                        return Ok(true);
                    }
                }
                false
            }
            DeclaredType::Record(c) => {
                for (_, c) in c {
                    if contains_computational(c, budget, location)? {
                        return Ok(true);
                    }
                }
                false
            }
            _ => false,
        })
    })
}
