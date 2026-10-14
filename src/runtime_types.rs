use std::ops::Range;

use crate::{
    ArrayType, Budget, ConstructedValue, EvaluationError, Located, Location, NominalTypeId,
    ProgramHost, QueryError, QueryErrorCategory, SetElementType, ValueType, constructed_value_type,
};

/// An opt-in interface contract, distinct from discovered [`ValueType`] facts.
/// Sequences and keyed cells are homogeneous; tuples have fixed positions and
/// records have exactly the declared labels, in any stored order. Keyed axes
/// remain runtime facts. Sets use the existing supported scalar identities.
/// `Native` uses the adapter's own kind identity, without erasing its values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclaredType<K> {
    Boolean,
    Integer,
    Float,
    String,
    Member(NominalTypeId),
    Sequence(Box<Self>),
    Tuple(Vec<Self>),
    Record(Vec<(String, Self)>),
    Set(SetElementType),
    Keyed(Box<Self>),
    Native(K),
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
        host.admit_native(value, expected, context, budget, location)
    })
}

fn admit_constructed_value<K>(
    value: &ConstructedValue,
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: Location,
) -> Result<(), QueryError> {
    // These facts are freshly discovered from actual data, never supplied tags.
    let actual = constructed_value_type(value, budget, location.clone())?;
    check_type(&actual, expected, budget, &location)
}

fn validate_declaration<K>(
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match expected {
            DeclaredType::Sequence(element) | DeclaredType::Keyed(element) => {
                validate_declaration(element, budget, location)?;
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
            DeclaredType::Boolean
            | DeclaredType::Integer
            | DeclaredType::Float
            | DeclaredType::String
            | DeclaredType::Member(_)
            | DeclaredType::Set(_)
            | DeclaredType::Native(_) => {}
        }
        Ok(())
    })
}

fn check_type<K>(
    actual: &ValueType,
    expected: &DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match (actual, expected) {
            (ValueType::Boolean, DeclaredType::Boolean)
            | (ValueType::Integer, DeclaredType::Integer)
            | (ValueType::Float, DeclaredType::Float)
            | (ValueType::String, DeclaredType::String) => Ok(()),
            (ValueType::Member(actual), DeclaredType::Member(expected)) if actual == expected => {
                Ok(())
            }
            (ValueType::Set(actual), DeclaredType::Set(expected)) if *actual == Some(*expected) => {
                Ok(())
            }
            (ValueType::Array(ArrayType::Positions(elements)), DeclaredType::Sequence(element)) => {
                check_elements(elements, element, "sequence element", budget, location)
            }
            (ValueType::KeyedArray(actual), DeclaredType::Keyed(element)) => {
                check_elements(&actual.elements, element, "keyed cell", budget, location)
            }
            (ValueType::Tuple(actual), DeclaredType::Tuple(expected)) => {
                if actual.len() != expected.len() {
                    return Err(refusal("tuple length mismatch", budget, location)?);
                }
                for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    check_type(actual, expected, budget, location).map_err(|error| {
                        nested_error(error, "tuple position", Some(index), budget)
                    })?;
                }
                Ok(())
            }
            (ValueType::Record(actual), DeclaredType::Record(expected)) => {
                if actual.len() != expected.len() {
                    return Err(refusal("record labels mismatch", budget, location)?);
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
                        return Err(nested_error(
                            refusal("missing declared record field", budget, location)?,
                            label,
                            None,
                            budget,
                        ));
                    };
                    check_type(actual, expected, budget, location)
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
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    for (index, actual) in elements.iter().enumerate() {
        check_type(actual, expected, budget, location)
            .map_err(|error| nested_error(error, label, Some(index), budget))?;
    }
    Ok(())
}

impl<K> DeclaredType<K> {
    fn description(&self) -> &'static str {
        match self {
            Self::Boolean => "expected Boolean",
            Self::Integer => "expected integer",
            Self::Float => "expected finite float",
            Self::String => "expected string",
            Self::Member(_) => "expected member of declared nominal type",
            Self::Sequence(_) => "expected positional sequence",
            Self::Tuple(_) => "expected tuple",
            Self::Record(_) => "expected record",
            Self::Set(_) => "expected set of declared element type",
            Self::Keyed(_) => "expected keyed array",
            Self::Native(_) => "expected native value",
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
