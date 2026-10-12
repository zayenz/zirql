use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::mem::size_of;

use crate::{Bindings, Budget, Located, Location, QueryError};

/// Supported homogeneous set identities. Float identities use numeric equality,
/// so positive and negative zero denote the same element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetElementType {
    Boolean,
    Integer,
    Float,
}

/// Owned constructed data. Arrays have positive one-based positions. Tuples and
/// records retain declaration order; sets have unique elements in type order.
/// Public construction is checked by the budgeted entry points before use.
/// There is deliberately no deep `Clone`; use [`copy_constructed_value`].
#[derive(Debug, PartialEq)]
pub enum ConstructedValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
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

pub type ValueExpression = Located<ValueExpressionKind>;

/// A host-lowered pure expression. Every node retains its query byte range.
/// Hosts reject unlisted surface forms and decimal spellings that round a
/// nonzero literal to zero before creating this representation.
#[derive(Debug, PartialEq)]
pub enum ValueExpressionKind {
    Literal(ConstructedValue),
    Name(String),
    Array(Vec<ValueExpression>),
    Tuple(Vec<ValueExpression>),
    Record(Vec<(Located<String>, ValueExpression)>),
    Set(Vec<ValueExpression>),
    Unary {
        operator: ValueUnaryOperator,
        operand: Box<ValueExpression>,
    },
    Binary {
        operator: ValueBinaryOperator,
        left: Box<ValueExpression>,
        right: Box<ValueExpression>,
    },
    Field {
        value: Box<ValueExpression>,
        field: Located<String>,
    },
    TupleAccess {
        value: Box<ValueExpression>,
        position: Located<usize>,
    },
    ArrayAccess {
        array: Box<ValueExpression>,
        index: Box<ValueExpression>,
    },
    IndexSet(Box<ValueExpression>),
    Range {
        lower: Box<ValueExpression>,
        upper: Box<ValueExpression>,
    },
    /// Conditions are checked in order. The final else is mandatory.
    Conditional {
        branches: Vec<(ValueExpression, ValueExpression)>,
        otherwise: Box<ValueExpression>,
    },
    Comprehension {
        constructor: ValueConstructor,
        clauses: Vec<Located<ValueGenerator>>,
        head: Box<ValueExpression>,
    },
}

/// Ordered generator clauses. None discards a value without installing a name.
/// Grouped `In` evaluates its source once and varies the last binder fastest.
#[derive(Debug, PartialEq)]
pub enum ValueGenerator {
    In {
        binders: Vec<Located<Option<String>>>,
        source: ValueExpression,
        filter: Option<ValueExpression>,
    },
    Bind {
        binder: Located<Option<String>>,
        value: ValueExpression,
        filter: Option<ValueExpression>,
    },
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
    Analysis::new(Globals::Types(binding_types)).walk(expression, budget)?;
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
    let mut analysis = Analysis::new(Globals::Types(binding_types));
    analysis.walk(expression, budget)?;
    Ok(analysis.free)
}

/// Discover exact concrete type facts while checking externally built values.
/// This includes finite floats, record labels and homogeneous sorted sets.
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
    let mut analysis = Analysis::new(Globals::Available(bindings));
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
    run(expression, bindings, &mut Vec::new(), budget)
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

fn scalar_set_type(value: &ConstructedValue) -> Option<SetElementType> {
    match value {
        ConstructedValue::Boolean(_) => Some(SetElementType::Boolean),
        ConstructedValue::Integer(_) => Some(SetElementType::Integer),
        ConstructedValue::Float(_) => Some(SetElementType::Float),
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
    Available(&'a Bindings<ConstructedValue>),
}

struct Analysis<'a> {
    globals: Globals<'a>,
    locals: Vec<(&'a str, Vec<ValueType>)>,
    free: Vec<Located<String>>,
}

impl<'a> Analysis<'a> {
    fn new(globals: Globals<'a>) -> Self {
        Self {
            globals,
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
            Globals::Available(bindings) => bindings.get(name, range).is_ok(),
        })
    }

    fn name(
        &mut self,
        name: &str,
        expression: &ValueExpression,
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
        admit_diagnostic("undefined binding: ", name, budget, &location)?;
        let ty = match self.globals {
            Globals::Types(types) => copy_type(
                types
                    .get(name)
                    .ok_or_else(|| error(&location, format!("undefined binding: {name}")))?,
                budget,
                &location,
            )?,
            Globals::Available(bindings) => {
                bindings.get(name, expression.range.clone())?;
                ValueType::Unestablished
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
        one_type(ty, budget, &location)
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
        expression: &'a ValueExpression,
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
        expression: &'a ValueExpression,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<ValueType>, QueryError> {
        match &expression.value {
            ValueExpressionKind::Literal(value) => {
                let ty = constructed_value_type(value, budget, location.clone())?;
                one_type(ty, budget, location)
            }
            ValueExpressionKind::Name(name) => self.name(name, expression, budget),
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
                for ty in self.walk(index, budget)? {
                    require_type(&ty, ScalarRequirement::Integer, location)?;
                }
                let constant = match index.value {
                    ValueExpressionKind::Literal(ConstructedValue::Integer(value)) => Some(value),
                    _ => None,
                };
                let mut result = Vec::new();
                for array in arrays {
                    match array {
                        ValueType::Array(_) if constant.is_some_and(|position| position <= 0) => {
                            push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                        }
                        ValueType::Array(ArrayType::Positions(entries)) if constant.is_some() => {
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
                        ValueType::Array(
                            ArrayType::Positions(entries) | ArrayType::Elements(entries),
                        ) => append_types(&mut result, entries, budget, location)?,
                        ValueType::Unestablished => {
                            push_metadata(&mut result, ValueType::Unestablished, budget, location)?
                        }
                        _ => return Err(error(location, "positional access needs an array")),
                    }
                }
                if result.is_empty() {
                    push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
                }
                Ok(result)
            }
            ValueExpressionKind::IndexSet(array) => {
                for ty in self.walk(array, budget)? {
                    budget.charge(1, location.clone())?;
                    if !matches!(ty, ValueType::Array(_) | ValueType::Unestablished) {
                        return Err(error(location, "index_set needs a positional array"));
                    }
                }
                one_type(
                    ValueType::Set(Some(SetElementType::Integer)),
                    budget,
                    location,
                )
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
                                    return Err(error(
                                        &clause_location,
                                        "in generator needs a binder",
                                    ));
                                }
                                let source_types = self.walk(source, budget)?;
                                let mut elements = Vec::new();
                                for ty in source_types {
                                    match ty {
                                        ValueType::Array(
                                            ArrayType::Positions(entries)
                                            | ArrayType::Elements(entries),
                                        ) => append_types(
                                            &mut elements,
                                            entries,
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

    fn condition(
        &mut self,
        condition: &'a ValueExpression,
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
        children: impl Iterator<Item = &'a ValueExpression>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<Vec<ValueType>>, QueryError> {
        let mut positions = metadata_vec(1, budget, location)?;
        positions.push(Vec::new());
        for child in children {
            let types = self.walk(child, budget)?;
            let count = positions
                .len()
                .checked_mul(types.len())
                .ok_or_else(|| error(location, "type possibilities overflow"))?;
            let mut expanded = metadata_vec(count, budget, location)?;
            for position in &positions {
                for ty in &types {
                    let mut entries = copy_types(position, budget, location)?;
                    let ty = copy_type(ty, budget, location)?;
                    push_metadata(&mut entries, ty, budget, location)?;
                    expanded.push(entries);
                }
            }
            positions = expanded;
        }
        Ok(positions)
    }
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

fn run<'a>(
    expression: &'a ValueExpression,
    bindings: &Bindings<ConstructedValue>,
    locals: &mut Vec<(&'a str, ConstructedValue)>,
    budget: &mut Budget,
) -> Result<ConstructedValue, QueryError> {
    let location = Location::Query(expression.range.clone());
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match &expression.value {
            ValueExpressionKind::Literal(value) => copy_constructed_value(value, budget, location),
            ValueExpressionKind::Name(name) => {
                for (local, value) in locals.iter().rev() {
                    budget.charge(
                        sum(1, name.len().min(local.len()), &location)?,
                        location.clone(),
                    )?;
                    if name == local {
                        return copy_constructed_value(value, budget, location);
                    }
                }
                admit_diagnostic("undefined binding: ", name, budget, &location)?;
                copy_constructed_value(
                    bindings.get(name, expression.range.clone())?,
                    budget,
                    location,
                )
            }
            ValueExpressionKind::Array(children)
            | ValueExpressionKind::Tuple(children)
            | ValueExpressionKind::Set(children) => {
                let mut values = admit_container(children.len(), budget, &location)?;
                let mut element_type = None;
                for child in children {
                    let value = run(child, bindings, locals, budget)?;
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
            ValueExpressionKind::Record(fields) => {
                let mut values = admit_container(fields.len(), budget, &location)?;
                for (label, child) in fields {
                    budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
                    let value = run(child, bindings, locals, budget)?;
                    values.push((label.value.to_owned(), value));
                }
                Ok(ConstructedValue::Record(values))
            }
            ValueExpressionKind::Unary { operator, operand } => unary(
                *operator,
                run(operand, bindings, locals, budget)?,
                budget,
                &location,
            ),
            ValueExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = run(left, bindings, locals, budget)?;
                if matches!(operator, ValueBinaryOperator::And | ValueBinaryOperator::Or) {
                    let a = match left {
                        ConstructedValue::Boolean(value) => value,
                        _ => return Err(error(&location, "condition needs a fixed Boolean")),
                    };
                    if (*operator == ValueBinaryOperator::And && !a)
                        || (*operator == ValueBinaryOperator::Or && a)
                    {
                        return scalar(ConstructedValue::Boolean(a), budget, &location);
                    }
                    let right = run(right, bindings, locals, budget)?;
                    return binary(
                        *operator,
                        ConstructedValue::Boolean(a),
                        right,
                        budget,
                        &location,
                    );
                }
                let right = run(right, bindings, locals, budget)?;
                binary(*operator, left, right, budget, &location)
            }
            ValueExpressionKind::Field { value, field } => {
                let ConstructedValue::Record(fields) = run(value, bindings, locals, budget)? else {
                    return Err(error(&location, "field access needs a record"));
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
                Err(error(
                    &location,
                    format!("missing record field: {}", field.value),
                ))
            }
            ValueExpressionKind::TupleAccess { value, position } => {
                let ConstructedValue::Tuple(values) = run(value, bindings, locals, budget)? else {
                    return Err(error(&location, "tuple access needs a tuple"));
                };
                take_position(values, position.value, budget, &location)
            }
            ValueExpressionKind::ArrayAccess { array, index } => {
                let array = run(array, bindings, locals, budget)?;
                let position = integer(
                    run(index, bindings, locals, budget)?,
                    &Location::Query(index.range.clone()),
                )?;
                let ConstructedValue::Array(values) = array else {
                    return Err(error(&location, "positional access needs an array"));
                };
                let position = usize::try_from(position)
                    .map_err(|_| error(&location, "missing array position"))?;
                take_position(values, position, budget, &location)
            }
            ValueExpressionKind::IndexSet(array) => {
                let ConstructedValue::Array(values) = run(array, bindings, locals, budget)? else {
                    return Err(error(&location, "index_set needs a positional array"));
                };
                let len =
                    i64::try_from(values.len()).map_err(|_| error(&location, "index overflow"))?;
                integer_range(1, len, budget, &location)
            }
            ValueExpressionKind::Range { lower, upper } => {
                let lower = integer(run(lower, bindings, locals, budget)?, &location)?;
                let upper = integer(run(upper, bindings, locals, budget)?, &location)?;
                integer_range(lower, upper, budget, &location)
            }
            ValueExpressionKind::Conditional {
                branches,
                otherwise,
            } => {
                for (condition, value) in branches {
                    if boolean(
                        run(condition, bindings, locals, budget)?,
                        &Location::Query(condition.range.clone()),
                    )? {
                        return run(value, bindings, locals, budget);
                    }
                }
                run(otherwise, bindings, locals, budget)
            }
            ValueExpressionKind::Comprehension {
                constructor,
                clauses,
                head,
            } => {
                let mut values = admit_container(0, budget, &location)?;
                let mut element_type = None;
                expand(
                    *constructor,
                    clauses,
                    head,
                    bindings,
                    locals,
                    &mut values,
                    &mut element_type,
                    budget,
                    &location,
                )?;
                Ok(match constructor {
                    ValueConstructor::Array => ConstructedValue::Array(values),
                    ValueConstructor::Set => ConstructedValue::Set {
                        element_type,
                        elements: values,
                    },
                })
            }
        }
    })
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

// This is the runtime clause chain, not a second expression evaluator. Each
// source runs before its own binders; scope restoration covers every Result.
#[expect(
    clippy::too_many_arguments,
    reason = "one clause chain retains its output and scope under the active budget"
)]
fn expand<'a>(
    constructor: ValueConstructor,
    clauses: &'a [Located<ValueGenerator>],
    head: &'a ValueExpression,
    bindings: &Bindings<ConstructedValue>,
    locals: &mut Vec<(&'a str, ConstructedValue)>,
    output: &mut Vec<ConstructedValue>,
    element_type: &mut Option<SetElementType>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    let Some((clause, rest)) = clauses.split_first() else {
        let value = run(head, bindings, locals, budget)?;
        match constructor {
            ValueConstructor::Array => {
                allocation::<ConstructedValue>(sum(output.len(), 1, location)?, location)?;
                budget.charge_results(1, location.clone())?;
                output.push(value);
            }
            ValueConstructor::Set => {
                insert_set(output, element_type, value, true, budget, location)?
            }
        }
        return Ok(());
    };
    let clause_location = Location::Query(clause.range.clone());
    budget.within_evaluation(1, clause_location.clone(), |budget| {
        budget.charge(1, clause_location.clone())?;
        match &clause.value {
            ValueGenerator::Bind {
                binder,
                value,
                filter,
            } => {
                budget.charge(1, clause_location.clone())?;
                let value = run(value, bindings, locals, budget)?;
                let saved = locals.len();
                let result = (|| {
                    if let Some(name) = &binder.value {
                        push_metadata(locals, (name, value), budget, &clause_location)?;
                    }
                    if filter_passes(filter, bindings, locals, budget)? {
                        expand(
                            constructor,
                            rest,
                            head,
                            bindings,
                            locals,
                            output,
                            element_type,
                            budget,
                            location,
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
                let source_value = run(source, bindings, locals, budget)?;
                let values = match source_value {
                    ConstructedValue::Array(values)
                    | ConstructedValue::Set {
                        elements: values, ..
                    } => values,
                    _ => {
                        return Err(error(
                            &Location::Query(source.range.clone()),
                            "in source needs an array or supported set",
                        ));
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
                    let result = (|| {
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
                        if filter_passes(filter, bindings, locals, budget)? {
                            expand(
                                constructor,
                                rest,
                                head,
                                bindings,
                                locals,
                                output,
                                element_type,
                                budget,
                                location,
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
    })
}

fn filter_passes<'a>(
    filter: &'a Option<ValueExpression>,
    bindings: &Bindings<ConstructedValue>,
    locals: &mut Vec<(&'a str, ConstructedValue)>,
    budget: &mut Budget,
) -> Result<bool, QueryError> {
    match filter {
        Some(filter) => boolean(
            run(filter, bindings, locals, budget)?,
            &Location::Query(filter.range.clone()),
        ),
        None => Ok(true),
    }
}
