//! The same closed patterns bind lets, typed parameters and selected match arms.
use super::execution::{Data, Host, NativeStep, Value};
use super::*;

pub type Pattern = Located<PatternKind>;
type BindingFacts<'a> = Vec<(&'a Located<String>, Vec<ValueType>)>;
type PatternBindings<'a, N, K> = Vec<(&'a str, Value<'a, N, K>)>;

/// Records inspect stored fields. `ignore_rest` explicitly permits other labels;
/// it never creates a rest value. Native structure requires an explicit host view.
#[derive(Debug, PartialEq)]
pub enum PatternKind {
    Name(Located<String>),
    Wildcard,
    Tuple(Vec<Pattern>),
    Record {
        fields: Vec<(Located<String>, Pattern)>,
        ignore_rest: bool,
    },
    Literal(ConstructedValue),
    Present(Box<Pattern>),
    Absent,
}

#[derive(Clone, Copy)]
pub(crate) enum PatternRef<'a> {
    Name(&'a Located<String>),
    Pattern(&'a Pattern),
}
impl<'a> PatternRef<'a> {
    pub fn range(self) -> &'a std::ops::Range<usize> {
        match self {
            Self::Name(n) => &n.range,
            Self::Pattern(p) => &p.range,
        }
    }
}

pub(crate) fn names<'a>(
    pattern: &'a Pattern,
    result: &mut Vec<&'a str>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    for (name, _) in facts(pattern, &[ValueType::Unestablished], budget)? {
        push_metadata(
            result,
            name.value.as_str(),
            budget,
            &Location::Query(name.range.clone()),
        )?;
    }
    Ok(())
}

/// Read bound names in written pattern order, borrowing their original ranges.
/// Validation and metadata allocations share the active allowance. No value is
/// matched and no native structure is inspected.
///
/// # Errors
/// Invalid pattern literals/fields, nesting and metadata limits are located.
/// The lexical owner still checks duplicate names and outer-scope collisions.
pub fn pattern_names<'a>(
    pattern: &'a Pattern,
    budget: &mut Budget,
) -> Result<Vec<&'a Located<String>>, QueryError> {
    let mut names = Vec::new();
    for (name, _) in facts(pattern, &[ValueType::Unestablished], budget)? {
        push_metadata(
            &mut names,
            name,
            budget,
            &Location::Query(name.range.clone()),
        )?;
    }
    Ok(names)
}

pub(crate) fn facts<'a>(
    pattern: &'a Pattern,
    types: &[ValueType],
    budget: &mut Budget,
) -> Result<BindingFacts<'a>, QueryError> {
    let location = Location::Query(pattern.range.clone());
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        let mut result = Vec::new();
        match &pattern.value {
            PatternKind::Name(name) => {
                budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
                if name.value == "_" {
                    return Err(error(&location, "use a wildcard to discard a value"));
                }
                push_metadata(
                    &mut result,
                    (name, copy_types(types, budget, &location)?),
                    budget,
                    &location,
                )?;
            }
            PatternKind::Wildcard | PatternKind::Absent => {}
            PatternKind::Literal(value) => {
                constructed_value_type(value, budget, location.clone())?;
                if !matches!(
                    value,
                    ConstructedValue::Boolean(_)
                        | ConstructedValue::Integer(_)
                        | ConstructedValue::Float(_)
                        | ConstructedValue::String(_)
                        | ConstructedValue::Member(_)
                ) {
                    return Err(error(&location, "literal pattern needs a present scalar"));
                }
            }
            PatternKind::Present(child) => {
                let mut present = Vec::new();
                for ty in types {
                    let ty = match copy_type(ty, budget, &location)? {
                        ValueType::Absent => continue,
                        ValueType::Declared {
                            actual,
                            declaration: DeclaredType::Optional(child),
                        } => ValueType::Declared {
                            actual: if matches!(actual.as_ref(), ValueType::Absent) {
                                Box::new(ValueType::Unestablished)
                            } else {
                                actual
                            },
                            declaration: *child,
                        },
                        other => other,
                    };
                    push_metadata(&mut present, ty, budget, &location)?;
                }
                if present.is_empty() {
                    push_metadata(&mut present, ValueType::Unestablished, budget, &location)?;
                }
                result = facts(child, &present, budget)?;
            }
            PatternKind::Tuple(children) => {
                for (position, child) in children.iter().enumerate() {
                    let child_types = child_facts(types, Some(position), None, budget, &location)?;
                    for binding in facts(child, &child_types, budget)? {
                        push_metadata(&mut result, binding, budget, &location)?;
                    }
                }
            }
            PatternKind::Record { fields, .. } => {
                for (index, (label, child)) in fields.iter().enumerate() {
                    budget.charge(label.value.len() + 1, Location::Query(label.range.clone()))?;
                    for (previous, _) in &fields[..index] {
                        budget.charge(
                            1 + previous.value.len().min(label.value.len()),
                            Location::Query(label.range.clone()),
                        )?;
                        if previous.value == label.value {
                            return Err(error(
                                &Location::Query(label.range.clone()),
                                "record pattern field is already defined",
                            ));
                        }
                    }
                    let child_types =
                        child_facts(types, None, Some(&label.value), budget, &location)?;
                    for binding in facts(child, &child_types, budget)? {
                        push_metadata(&mut result, binding, budget, &location)?;
                    }
                }
            }
        }
        // Name conflicts are checked by the lexical owner, including names in
        // separate parameters. Admit the collected metadata before growth above.
        Ok(result)
    })
}
fn child_facts(
    types: &[ValueType],
    position: Option<usize>,
    label: Option<&str>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<ValueType>, QueryError> {
    let mut result = Vec::new();
    for ty in types {
        let ty = shape_type(copy_type(ty, budget, location)?, budget, location)?;
        let child = match (&ty, position, label) {
            (ValueType::Tuple(children), Some(i), _) => children.get(i),
            (ValueType::Record(fields), _, Some(label)) => {
                let mut child = None;
                for (name, ty) in fields {
                    budget.charge(1 + name.len().min(label.len()), location.clone())?;
                    if name == label {
                        child = Some(ty);
                        break;
                    }
                }
                child
            }
            _ => None,
        };
        push_metadata(
            &mut result,
            match child {
                Some(ty) => copy_type(ty, budget, location)?,
                None => ValueType::Unestablished,
            },
            budget,
            location,
        )?;
    }
    if result.is_empty() {
        push_metadata(&mut result, ValueType::Unestablished, budget, location)?;
    }
    Ok(result)
}

/// Stage all names before the caller changes its lexical slots. Ordinary shape,
/// literal and presence mismatches are separate from host/admission/work errors.
#[expect(
    clippy::type_complexity,
    reason = "Staged bindings retain the adapter native value, contract kind and error types"
)]
pub(crate) fn bind<'a, Q, H: Host<'a, Q>>(
    pattern: PatternRef<'a>,
    value: Value<'a, H::Native, H::Kind>,
    host: &mut H,
    budget: &mut Budget,
) -> Result<Option<PatternBindings<'a, H::Native, H::Kind>>, H::Error> {
    let location = Location::Query(pattern.range().clone());
    let mut pending = metadata_vec(1, budget, &location)?;
    pending.push((pattern, value));
    let mut bindings = Vec::new();
    while let Some((pattern, mut value)) = pending.pop() {
        let location = Location::Query(pattern.range().clone());
        budget.charge(1, location.clone())?;
        let pattern = match pattern {
            PatternRef::Name(name) => {
                push_metadata(
                    &mut bindings,
                    (name.value.as_str(), value),
                    budget,
                    &location,
                )?;
                continue;
            }
            PatternRef::Pattern(pattern) => pattern,
        };
        match &pattern.value {
            PatternKind::Name(name) => push_metadata(
                &mut bindings,
                (name.value.as_str(), value),
                budget,
                &location,
            )?,
            PatternKind::Wildcard => {}
            PatternKind::Absent | PatternKind::Present(_) => {
                let present = host.occurs(&value, budget, &location)?;
                match &pattern.value {
                    PatternKind::Absent if present => return Ok(None),
                    PatternKind::Present(child) if present => {
                        if let Some(DeclaredType::Optional(inner)) = value.contract {
                            value.contract = Some(inner);
                        }
                        let declaration = if matches!(value.data, Data::Native(_)) {
                            host.constructed(&value, budget, &location)?
                                .and_then(|v| v.declaration)
                                .map(underlying)
                                .map(|d| copy_declaration(d, budget, &location))
                                .transpose()?
                        } else {
                            None
                        };
                        match &mut value.data {
                            Data::Constructed(_) => {
                                let Data::Constructed(data) = value.data else {
                                    unreachable!()
                                };
                                value.data =
                                    Data::Constructed(data.into_present(budget, &location)?);
                            }
                            Data::Native(view) => {
                                if let Some(d) = declaration {
                                    view.retain_declaration(
                                        std::sync::Arc::new(d),
                                        budget,
                                        &location,
                                    )?;
                                }
                            }
                        }
                        push_metadata(
                            &mut pending,
                            (PatternRef::Pattern(child), value),
                            budget,
                            &location,
                        )?;
                    }
                    PatternKind::Present(_) => return Ok(None),
                    _ => {}
                }
            }
            PatternKind::Literal(literal) => {
                let Some(data) = host.constructed(&value, budget, &location)? else {
                    return Ok(None);
                };
                let actual = data.value.data();
                let compatible = matches!(
                    (literal, actual),
                    (ConstructedValue::Boolean(_), ConstructedValue::Boolean(_))
                        | (ConstructedValue::Integer(_), ConstructedValue::Integer(_))
                        | (ConstructedValue::Float(_), ConstructedValue::Float(_))
                        | (ConstructedValue::String(_), ConstructedValue::String(_))
                ) || matches!((literal, actual), (ConstructedValue::Member(a), ConstructedValue::Member(b)) if a.type_id == b.type_id);
                if !compatible || !equal_values(literal, actual, budget, &location)? {
                    return Ok(None);
                }
            }
            PatternKind::Tuple(children) => {
                let len = match host.constructed(&value, budget, &location)? {
                    Some(data) => match data.value.data() {
                        ConstructedValue::Tuple(v) => Some(v.len()),
                        _ => None,
                    },
                    None => {
                        let Data::Native(view) = &value.data else {
                            unreachable!()
                        };
                        host.native_tuple(host.native_data(view, budget, &location)?)
                            .map(<[_]>::len)
                    }
                };
                if len != Some(children.len()) {
                    return Ok(None);
                }
                for (index, child) in children.iter().enumerate().rev() {
                    let child_value = select(
                        &mut value,
                        NativeStep::Tuple(index),
                        host,
                        budget,
                        &location,
                    )?;
                    push_metadata(
                        &mut pending,
                        (PatternRef::Pattern(child), child_value),
                        budget,
                        &location,
                    )?;
                }
            }
            PatternKind::Record {
                fields,
                ignore_rest,
            } => {
                let len = match host.constructed(&value, budget, &location)? {
                    Some(data) => match data.value.data() {
                        ConstructedValue::Record(v) => Some(v.len()),
                        _ => None,
                    },
                    None => {
                        let Data::Native(view) = &value.data else {
                            unreachable!()
                        };
                        host.native_record(host.native_data(view, budget, &location)?)
                            .map(|r| r.fields.len())
                    }
                };
                let Some(len) = len else {
                    return Ok(None);
                };
                if (!ignore_rest && len != fields.len()) || len < fields.len() {
                    return Ok(None);
                }
                // Select in written order, then push in reverse so bindings have
                // the same slot order as preparation.
                let mut selected = metadata_vec(fields.len(), budget, &location)?;
                for (label, child) in fields {
                    let mut found = None;
                    for index in 0..len {
                        let actual = match host.constructed(&value, budget, &location)? {
                            Some(data) => {
                                let ConstructedValue::Record(fields) = data.value.data() else {
                                    unreachable!()
                                };
                                &fields[index].0
                            }
                            None => {
                                let Data::Native(view) = &value.data else {
                                    unreachable!()
                                };
                                &host
                                    .native_record(host.native_data(view, budget, &location)?)
                                    .expect("record view")
                                    .fields[index]
                                    .0
                            }
                        };
                        budget.charge(
                            1 + actual.len().min(label.value.len()),
                            Location::Query(label.range.clone()),
                        )?;
                        if actual == &label.value {
                            found = Some(index);
                            break;
                        }
                    }
                    let Some(index) = found else {
                        return Ok(None);
                    };
                    selected.push((
                        PatternRef::Pattern(child),
                        select(
                            &mut value,
                            NativeStep::Record(index),
                            host,
                            budget,
                            &location,
                        )?,
                    ));
                }
                for child in selected.into_iter().rev() {
                    push_metadata(&mut pending, child, budget, &location)?;
                }
            }
        }
    }
    Ok(Some(bindings))
}
pub(crate) fn select<'a, Q, H: Host<'a, Q> + ?Sized>(
    value: &mut Value<'a, H::Native, H::Kind>,
    step: NativeStep,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<Value<'a, H::Native, H::Kind>, H::Error> {
    let mut contract = value.contract;
    while let Some(DeclaredType::Optional(inner)) = contract {
        contract = Some(inner);
    }
    let contract = match (contract, &step) {
        (Some(DeclaredType::Tuple(children)), NativeStep::Tuple(index)) => children.get(*index),
        (Some(DeclaredType::Record(fields)), NativeStep::Record(index)) => {
            let label = match host.constructed(value, budget, location)? {
                Some(data) => {
                    let ConstructedValue::Record(fields) = data.value.data() else {
                        unreachable!()
                    };
                    &fields[*index].0
                }
                None => {
                    let Data::Native(view) = &value.data else {
                        unreachable!()
                    };
                    &host
                        .native_record(host.native_data(view, budget, location)?)
                        .expect("record view")
                        .fields[*index]
                        .0
                }
            };
            Some(record_declaration(fields, label, budget, location)?)
        }
        _ => None,
    };
    let constructed = if matches!(value.data, Data::Native(_)) {
        host.constructed(value, budget, location)?
    } else {
        None
    };
    let is_constructed = constructed.is_some();
    let constructed_child = match constructed {
        Some(data) => data
            .child(step.index(), budget, location)?
            .declaration
            .map(|d| copy_declaration(d, budget, location))
            .transpose()?,
        None => None,
    };
    let data = match &mut value.data {
        Data::Constructed(data) => {
            Data::Constructed(data.pattern_child(step.index(), budget, location)?)
        }
        Data::Native(view) => {
            let mut selected = view.child(
                if is_constructed {
                    NativeStep::Constructed(step.index())
                } else {
                    step
                },
                budget,
                location,
            )?;
            if let Some(d) = constructed_child {
                selected.retain_declaration(std::sync::Arc::new(d), budget, location)?;
            }
            Data::Native(selected)
        }
    };
    Ok(Value { data, contract })
}

/// Copy reusable pattern metadata under the active allowance, retaining every
/// original query range. This never reads or copies a matched native value.
///
/// # Errors
/// Metadata, scalar-copy and nesting limits fail at the original pattern.
pub fn copy_pattern(pattern: &Pattern, budget: &mut Budget) -> Result<Pattern, QueryError> {
    let location = Location::Query(pattern.range.clone());
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        budget.charge_results(1, location.clone())?;
        let copy_name = |name: &Located<String>, budget: &mut Budget| {
            budget.charge(name.value.len() + 1, Location::Query(name.range.clone()))?;
            Ok::<_, QueryError>(name.clone())
        };
        let value = match &pattern.value {
            PatternKind::Name(name) => PatternKind::Name(copy_name(name, budget)?),
            PatternKind::Wildcard => PatternKind::Wildcard,
            PatternKind::Absent => PatternKind::Absent,
            PatternKind::Literal(value) => {
                PatternKind::Literal(copy_constructed_value(value, budget, location.clone())?)
            }
            PatternKind::Present(child) => {
                PatternKind::Present(Box::new(copy_pattern(child, budget)?))
            }
            PatternKind::Tuple(children) => {
                let mut copied = Vec::new();
                for child in children {
                    let child = copy_pattern(child, budget)?;
                    push_metadata(&mut copied, child, budget, &location)?;
                }
                PatternKind::Tuple(copied)
            }
            PatternKind::Record {
                fields,
                ignore_rest,
            } => {
                let mut copied = Vec::new();
                for (label, child) in fields {
                    let label = copy_name(label, budget)?;
                    let child = copy_pattern(child, budget)?;
                    push_metadata(&mut copied, (label, child), budget, &location)?;
                }
                PatternKind::Record {
                    fields: copied,
                    ignore_rest: *ignore_rest,
                }
            }
        };
        Ok(Located::new(value, pattern.range.clone()))
    })
}

pub fn dispose_pattern(pattern: Pattern) {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern.value {
            PatternKind::Tuple(children) => pending.extend(children),
            PatternKind::Record { fields, .. } => {
                pending.extend(fields.into_iter().map(|(_, p)| p))
            }
            PatternKind::Present(child) => pending.push(*child),
            PatternKind::Name(_)
            | PatternKind::Wildcard
            | PatternKind::Literal(_)
            | PatternKind::Absent => {}
        }
    }
}
