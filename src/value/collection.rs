//! Fixed ordinary collection operations. Keyed arrays only expose cardinality.
use super::*;

pub(crate) fn collection_len(
    value: &ConstructedValue,
    allow_keyed: bool,
    location: &Location,
) -> Result<usize, QueryError> {
    Ok(match value.data() {
        ConstructedValue::Array(values) => values.len(),
        ConstructedValue::Set { elements, .. } => elements.len(),
        ConstructedValue::Record(fields) => fields.len(),
        ConstructedValue::Dictionary(entries) => entries.len(),
        ConstructedValue::KeyedArray { entries, .. } if allow_keyed => entries.len(),
        _ => {
            return Err(error(
                location,
                "operation needs a positional array, set or dictionary",
            ));
        }
    })
}

/// Predicate subjects for ordinary collections: elements or dictionary values.
/// Declarations remain available even when the collection contains no entries.
///
/// # Errors
/// Refuses keyed arrays, tuples and scalars, and retains located budget failures.
pub fn collection_subject_types(
    types: &[ValueType],
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<ValueType>, QueryError> {
    let mut result = Vec::new();
    for ty in types {
        let original = copy_type(ty, budget, &location)?;
        let mut subjects = match shape_type(original, budget, &location)? {
            ValueType::Array(ArrayType::Positions(elements) | ArrayType::Elements(elements))
            | ValueType::ArrayKind(elements) => elements,
            ValueType::Set(element) => one_type(set_value_type(element), budget, &location)?,
            ValueType::Dictionary { values, .. } => values,
            ValueType::Record(fields) => {
                let mut values = metadata_vec(fields.len(), budget, &location)?;
                values.extend(fields.into_iter().map(|(_, value)| value));
                values
            }
            ValueType::RecordKind | ValueType::Unestablished => {
                one_type(ValueType::Unestablished, budget, &location)?
            }
            _ => {
                return Err(error(
                    &location,
                    "operation needs a positional array, set or dictionary",
                ));
            }
        };
        if subjects.is_empty()
            && let ValueType::Declared { declaration, .. } = ty
        {
            match underlying(declaration) {
                DeclaredType::Dictionary(_, value) => push_metadata(
                    &mut subjects,
                    declared_fact(value, ValueType::Unestablished, budget, &location)?,
                    budget,
                    &location,
                )?,
                DeclaredType::Record(fields) => {
                    for (_, value) in fields {
                        push_metadata(
                            &mut subjects,
                            declared_fact(value, ValueType::Unestablished, budget, &location)?,
                            budget,
                            &location,
                        )?;
                    }
                }
                _ => {}
            }
        }
        append_types(&mut result, subjects, budget, &location)?;
    }
    if result.is_empty() {
        push_metadata(&mut result, ValueType::Unestablished, budget, &location)?;
    }
    Ok(result)
}

/// Admit ordinary selection and retain contracts while weakening removed positions.
/// A removing operation makes structural record fields optional. Movement retains
/// their required schema; reverse updates known array positions.
///
/// # Errors
/// Refuses unsupported collection kinds and reports located budget exhaustion.
pub fn collection_selection_types(
    types: &[ValueType],
    removes: bool,
    reverses: bool,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<ValueType>, QueryError> {
    // Validate before copying facts, including empty and deferred collections.
    collection_subject_types(types, budget, location.clone())?;
    let mut result = metadata_vec(types.len(), budget, &location)?;
    for ty in types {
        result.push(selection_type(
            copy_type(ty, budget, &location)?,
            removes,
            reverses,
            budget,
            &location,
        )?);
    }
    Ok(result)
}

fn selection_type(
    ty: ValueType,
    removes: bool,
    reverses: bool,
    budget: &mut Budget,
    location: &Location,
) -> Result<ValueType, QueryError> {
    budget.charge(1, location.clone())?;
    Ok(match ty {
        ValueType::Declared {
            actual,
            mut declaration,
        } => {
            if removes {
                optional_record_fields(&mut declaration, budget, location)?;
            }
            ValueType::Declared {
                actual: Box::new(selection_type(
                    *actual, removes, reverses, budget, location,
                )?),
                declaration,
            }
        }
        ValueType::Array(ArrayType::Positions(mut elements)) => {
            if removes {
                ValueType::Array(ArrayType::Elements(elements))
            } else {
                if reverses {
                    elements.reverse();
                }
                ValueType::Array(ArrayType::Positions(elements))
            }
        }
        ValueType::Record(fields) if removes => {
            let mut result = metadata_vec(fields.len(), budget, location)?;
            for (name, ty) in fields {
                let (actual, declaration) = match ty {
                    ValueType::Declared {
                        actual,
                        declaration,
                    } => (*actual, declaration),
                    actual => (actual, DeclaredType::Value),
                };
                budget.charge(2, location.clone())?;
                result.push((
                    name,
                    ValueType::Declared {
                        actual: Box::new(actual),
                        declaration: DeclaredType::Optional(Box::new(declaration)),
                    },
                ));
            }
            ValueType::Record(result)
        }
        ty => ty,
    })
}

fn optional_record_fields(
    declaration: &mut DeclaredType<()>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    match declaration {
        DeclaredType::Optional(child) => optional_record_fields(child, budget, location)?,
        DeclaredType::Record(fields) => {
            for (_, child) in fields {
                if !matches!(child, DeclaredType::Optional(_)) {
                    budget.charge(1, location.clone())?;
                    *child = DeclaredType::Optional(Box::new(std::mem::replace(
                        child,
                        DeclaredType::Value,
                    )));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Restore the original container declaration after a host rebuilds its rows.
/// Removing fields weakens only their presence; element/key/value schemas survive.
///
/// # Errors
/// Declaration copying uses the active budget and supplied location.
pub fn restore_collection_declaration(
    original: &ConstructedValue,
    selected: ConstructedValue,
    removes: bool,
    budget: &mut Budget,
    location: Location,
) -> Result<ConstructedValue, QueryError> {
    let Some(declaration) = declaration(original) else {
        return Ok(selected);
    };
    let mut declaration = copy_declaration(declaration, budget, &location)?;
    if removes {
        optional_record_fields(&mut declaration, budget, &location)?;
    }
    budget.charge(1, location.clone())?;
    Ok(ConstructedValue::Declared {
        value: Box::new(selected.into_data()),
        declaration,
    })
}

pub(crate) fn collection_subject(
    value: &ConstructedValue,
    index: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let subject = match value.data() {
        ConstructedValue::Array(values) => &values[index],
        ConstructedValue::Set { elements, .. } => &elements[index],
        ConstructedValue::Record(fields) => &fields[index].1,
        ConstructedValue::Dictionary(entries) => &entries[index].1,
        _ => {
            return Err(error(
                location,
                "operation needs a positional array, set or dictionary",
            ));
        }
    };
    let subject = copy_constructed_value(subject, budget, location.clone())?;
    let child = match declaration(value).map(underlying) {
        Some(
            DeclaredType::Array(child)
            | DeclaredType::Sequence(child)
            | DeclaredType::Dictionary(_, child),
        ) => Some(child.as_ref()),
        Some(DeclaredType::Record(fields)) => match value.data() {
            ConstructedValue::Record(entries) => Some(record_declaration(
                fields,
                &entries[index].0,
                budget,
                location,
            )?),
            _ => None,
        },
        _ => None,
    };
    match child {
        Some(child) => annotate(
            subject,
            copy_declaration(child, budget, location)?,
            budget,
            location,
        ),
        None => Ok(subject),
    }
}

pub(crate) fn retain_collection(
    mut value: ConstructedValue,
    retained: &[bool],
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    if let ConstructedValue::Declared {
        value: child,
        declaration,
    } = &mut value
    {
        optional_record_fields(declaration, budget, location)?;
        **child = retain_collection(
            std::mem::replace(child.as_mut(), ConstructedValue::Absent),
            retained,
            budget,
            location,
        )?;
        return Ok(value);
    }
    budget.charge(retained.len(), location.clone())?;
    let mut index = 0;
    let mut keep = || {
        let keep = retained[index];
        index += 1;
        keep
    };
    match &mut value {
        ConstructedValue::Array(values) => values.retain(|_| keep()),
        ConstructedValue::Record(fields) => fields.retain(|_| keep()),
        ConstructedValue::Set { elements, .. } => {
            let mut values = std::mem::replace(elements, Vec::new().into()).into_vec();
            values.retain(|_| keep());
            *elements = values.into();
        }
        ConstructedValue::Dictionary(entries) => {
            // Reuse the existing entry allocation when moving the admitted pairs.
            let mut values = std::mem::replace(entries, Vec::new().into()).into_vec();
            values.retain(|_| keep());
            *entries = values.into();
        }
        _ => {
            return Err(error(
                location,
                "operation needs a positional array, set or dictionary",
            ));
        }
    }
    Ok(value)
}

pub(crate) fn select_collection(
    mut value: ConstructedValue,
    operation: &crate::Stage<()>,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let len = collection_len(&value, false, location)?;
    if let crate::Stage::Check { expected, message } = operation {
        crate::check_cardinality(
            len,
            *expected,
            message.as_deref(),
            match location {
                Location::Query(range) => range.clone(),
                _ => 0..0,
            },
        )?;
        return Ok(value);
    }
    budget.charge(len, location.clone())?;
    let removes = matches!(operation, crate::Stage::Head(_) | crate::Stage::Tail(_));
    let mut data = &mut value;
    while let ConstructedValue::Declared { value, declaration } = data {
        if removes {
            optional_record_fields(declaration, budget, location)?;
        }
        data = value;
    }
    match data {
        ConstructedValue::Array(values) => select_entries(values, operation),
        ConstructedValue::Set { elements, .. } => {
            let mut values = std::mem::replace(elements, Vec::new().into()).into_vec();
            select_entries(&mut values, operation);
            *elements = values.into();
        }
        ConstructedValue::Record(fields) => select_entries(fields, operation),
        ConstructedValue::Dictionary(entries) => {
            let mut values = std::mem::replace(entries, Vec::new().into()).into_vec();
            select_entries(&mut values, operation);
            *entries = values.into();
        }
        _ => unreachable!("admitted collection"),
    }
    Ok(value)
}

fn select_entries<T>(entries: &mut Vec<T>, operation: &crate::Stage<()>) {
    match operation {
        crate::Stage::Head(count) => entries.truncate(*count),
        crate::Stage::Tail(count) => crate::tail(entries, *count),
        crate::Stage::Reverse => crate::reverse(entries),
        _ => unreachable!("fixed selection operation"),
    }
}
