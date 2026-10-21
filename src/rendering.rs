//! Explicit JSON and Markdown rendering after evaluation.

mod markdown;
pub use markdown::render_markdown;

use std::io::{self, Write};

use serde_json::{Map, Value};

use crate::value::allocation;
use crate::{
    Budget, ConstructedValue, KeyAxis, KeyedEntry, Location, NominalMember, QueryError,
    QueryErrorCategory, ValueKey, constructed_value_type,
};

/// The ordinary report format or the explicit tagged data format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonRepresentation {
    /// Scalars, ordered arrays and records; dictionaries retain recursive keys
    /// in entry pairs. Members and keyed arrays use their tagged representation.
    Ordinary,
    /// Explicit scalar/collection kinds, nominal names and complete array axes.
    /// Records remain ordered objects containing tagged values.
    Tagged,
}

/// Whitespace used by the buffered JSON renderers. Neither layout adds a newline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonLayout {
    Compact,
    Pretty,
}

/// Convert borrowed constructed data into an insertion-ordered JSON tree.
/// Declared wrappers are transparent. General dictionary keys retain their
/// structure; sparse keyed arrays retain unused axis keys and only present cells.
/// Tagged nominal names do not reconstruct native identity.
///
/// This is an explicit post-evaluation operation. Native projections and host
/// document/completeness envelopes remain with the host. Computational values
/// are outside [`ConstructedValue`] and cannot be passed to this function.
///
/// # Errors
/// Checks externally constructed values through [`constructed_value_type`].
/// Malformed data, nonfinite floats and exhausted allowances retain `location`.
/// Traversal, JSON nodes/slots and copied text use the supplied active budget;
/// failed calls do not refund spending or return a partial tree.
pub fn to_json_value(
    value: &ConstructedValue,
    representation: JsonRepresentation,
    budget: &mut Budget,
    location: Location,
) -> Result<Value, QueryError> {
    constructed_value_type(value, budget, location.clone())?;
    JsonConversion {
        budget,
        location: &location,
    }
    .value(value, representation)
}

/// Convert constructed data and render a complete UTF-8 JSON buffer.
///
/// ```
/// use zirql::{Budget, ConstructedValue as V, JsonLayout, JsonRepresentation,
///             Location, ProgramLimits, render_json};
/// let mut budget = Budget::for_program(ProgramLimits {
///     work: 10_000, results: 10_000, nesting: 64, stream_items: 100,
/// });
/// let bytes = render_json(
///     &V::Record(vec![("count".into(), V::Integer(3))]),
///     JsonRepresentation::Ordinary, JsonLayout::Compact,
///     &mut budget, Location::Query(0..5),
/// )?;
/// assert_eq!(bytes, br#"{"count":3}"#);
/// # Ok::<(), zirql::QueryError>(())
/// ```
///
/// # Errors
/// Returns conversion or output failures with the supplied location and budget
/// category. The buffer is returned only on success. See [`to_json_value`] and
/// [`render_json_value`] for accounting and representation details.
pub fn render_json(
    value: &ConstructedValue,
    representation: JsonRepresentation,
    layout: JsonLayout,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<u8>, QueryError> {
    let value = to_json_value(value, representation, budget, location.clone())?;
    render_json_value(&value, layout, budget, location)
}

/// Render an existing JSON tree, including a host-owned document envelope.
/// Object and array order is preserved. No framing newline is added.
///
/// # Errors
/// Traversal uses the budget's active nesting/reentry guard. Each actual escaped
/// output byte spends work and result allowance before buffer growth. Failures
/// retain `location` and the original limit category, and return no partial bytes.
pub fn render_json_value(
    value: &Value,
    layout: JsonLayout,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<u8>, QueryError> {
    visit_json(value, budget, &location)?;
    let mut writer = OutputBuffer {
        bytes: Vec::new(),
        budget,
        location: &location,
        error: None,
    };
    let result = match layout {
        JsonLayout::Compact => serde_json::to_writer(&mut writer, value),
        JsonLayout::Pretty => serde_json::to_writer_pretty(&mut writer, value),
    };
    result.map_err(|error| {
        writer
            .error
            .take()
            .unwrap_or_else(|| failure(&location, format!("writing JSON: {error}")))
    })?;
    Ok(writer.bytes)
}

struct JsonConversion<'a> {
    budget: &'a mut Budget,
    location: &'a Location,
}

impl JsonConversion<'_> {
    fn value(
        &mut self,
        value: &ConstructedValue,
        representation: JsonRepresentation,
    ) -> Result<Value, QueryError> {
        let location = self.location;
        self.budget
            .within_evaluation(1, location.clone(), |budget| {
                budget.charge(1, location.clone())?;
                JsonConversion { budget, location }.convert(value.data(), representation)
            })
    }

    fn convert(
        &mut self,
        value: &ConstructedValue,
        representation: JsonRepresentation,
    ) -> Result<Value, QueryError> {
        use ConstructedValue as V;
        let tagged = representation == JsonRepresentation::Tagged;
        match value {
            V::Declared { .. } => unreachable!("declared wrappers are transparent"),
            V::Member(member) => self.member(member),
            V::KeyedArray { axes, entries } => self.keyed(axes, entries),
            V::Record(fields) => {
                let mut object = self.object(fields.len())?;
                for (label, value) in fields {
                    let value = self.value(value, representation)?;
                    self.insert(&mut object, label, value)?;
                }
                Ok(Value::Object(object))
            }
            V::Dictionary(entries) => {
                let mut object = self.tag("dictionary", 2)?;
                let mut output = self.array(entries.len())?;
                for (key, value) in entries {
                    let mut entry = self.object(2)?;
                    let key = self.value(key, representation)?;
                    self.insert(&mut entry, "key", key)?;
                    let value = self.value(value, representation)?;
                    self.insert(&mut entry, "value", value)?;
                    output.push(Value::Object(entry));
                }
                self.insert(&mut object, "entries", Value::Array(output))?;
                Ok(Value::Object(object))
            }
            V::Array(values) if tagged => self.positional(values),
            V::Array(_) | V::Tuple(_) | V::Set { .. } => {
                let values: &[V] = match value {
                    V::Array(values) | V::Tuple(values) => values,
                    V::Set { elements, .. } => elements,
                    _ => unreachable!("matched element collection"),
                };
                let mut output = self.array(values.len())?;
                for value in values {
                    output.push(self.value(value, representation)?);
                }
                if tagged {
                    let kind = if matches!(value, V::Tuple(_)) {
                        "tuple"
                    } else {
                        "set"
                    };
                    let mut object = self.tag(kind, 2)?;
                    self.insert(&mut object, "elements", Value::Array(output))?;
                    Ok(Value::Object(object))
                } else {
                    Ok(Value::Array(output))
                }
            }
            scalar => {
                let kind = match scalar {
                    V::Absent => "absent",
                    V::Boolean(_) => "boolean",
                    V::Integer(_) => "integer",
                    V::Float(_) => "float",
                    V::String(_) => "string",
                    _ => unreachable!("containers are handled above"),
                };
                if !tagged {
                    return self.scalar(scalar);
                }
                let absent = matches!(scalar, V::Absent);
                let mut object = self.tag(kind, if absent { 1 } else { 2 })?;
                if !absent {
                    let value = self.scalar(scalar)?;
                    self.insert(&mut object, "value", value)?;
                }
                Ok(Value::Object(object))
            }
        }
    }

    fn scalar(&mut self, value: &ConstructedValue) -> Result<Value, QueryError> {
        use ConstructedValue as V;
        if let V::String(value) = value {
            return self.string(value);
        }
        self.admit(1)?;
        Ok(match value {
            V::Absent => Value::Null,
            V::Boolean(value) => Value::Bool(*value),
            V::Integer(value) => Value::Number((*value).into()),
            V::Float(value) => Value::Number(
                serde_json::Number::from_f64(*value)
                    .ok_or_else(|| failure(self.location, "nonfinite float"))?,
            ),
            _ => unreachable!("scalar conversion"),
        })
    }

    fn member(&mut self, member: &NominalMember) -> Result<Value, QueryError> {
        let mut object = self.tag("member", 3)?;
        let name = self.string(&member.type_name)?;
        self.insert(&mut object, "enum", name)?;
        let name = self.string(&member.member_name)?;
        self.insert(&mut object, "name", name)?;
        Ok(Value::Object(object))
    }

    fn key(&mut self, key: &ValueKey) -> Result<Value, QueryError> {
        match key {
            ValueKey::Integer(value) => self.value(
                &ConstructedValue::Integer(*value),
                JsonRepresentation::Tagged,
            ),
            ValueKey::Member(member) => self.member(member),
        }
    }

    fn keys(&mut self, keys: &[ValueKey]) -> Result<Value, QueryError> {
        let mut output = self.array(keys.len())?;
        for key in keys {
            output.push(self.key(key)?);
        }
        Ok(Value::Array(output))
    }

    fn keyed(&mut self, axes: &[KeyAxis], entries: &[KeyedEntry]) -> Result<Value, QueryError> {
        let mut object = self.tag("array", 4)?;
        let indexing = self.string("explicit")?;
        self.insert(&mut object, "indexing", indexing)?;
        let mut output_axes = self.array(axes.len())?;
        for axis in axes {
            output_axes.push(self.keys(&axis.keys)?);
        }
        self.insert(&mut object, "axes", Value::Array(output_axes))?;
        let mut output_entries = self.array(entries.len())?;
        for entry in entries {
            let mut output = self.object(2)?;
            let key = self.keys(&entry.key)?;
            self.insert(&mut output, "key", key)?;
            let value = self.value(&entry.value, JsonRepresentation::Tagged)?;
            self.insert(&mut output, "value", value)?;
            output_entries.push(Value::Object(output));
        }
        self.insert(&mut object, "entries", Value::Array(output_entries))?;
        Ok(Value::Object(object))
    }

    fn positional(&mut self, values: &[ConstructedValue]) -> Result<Value, QueryError> {
        i64::try_from(values.len())
            .map_err(|_| failure(self.location, "positional key overflow"))?;
        let mut object = self.tag("array", 4)?;
        let indexing = self.string("positional")?;
        self.insert(&mut object, "indexing", indexing)?;
        let mut axes = self.array(1)?;
        let mut axis = self.array(values.len())?;
        let mut entries = self.array(values.len())?;
        for (index, value) in values.iter().enumerate() {
            // The complete length was checked before allocating either expansion.
            let position = ValueKey::Integer(index as i64 + 1);
            axis.push(self.key(&position)?);
            let mut entry = self.object(2)?;
            let key = self.keys(std::slice::from_ref(&position))?;
            self.insert(&mut entry, "key", key)?;
            let value = self.value(value, JsonRepresentation::Tagged)?;
            self.insert(&mut entry, "value", value)?;
            entries.push(Value::Object(entry));
        }
        axes.push(Value::Array(axis));
        self.insert(&mut object, "axes", Value::Array(axes))?;
        self.insert(&mut object, "entries", Value::Array(entries))?;
        Ok(Value::Object(object))
    }

    fn admit(&mut self, units: usize) -> Result<(), QueryError> {
        self.budget.charge(units, self.location.clone())?;
        self.budget.charge_results(units, self.location.clone())
    }

    fn array(&mut self, length: usize) -> Result<Vec<Value>, QueryError> {
        allocation::<Value>(length, self.location)?;
        self.admit(
            length
                .checked_add(1)
                .ok_or_else(|| failure(self.location, "JSON size overflow"))?,
        )?;
        Ok(Vec::with_capacity(length))
    }

    fn object(&mut self, length: usize) -> Result<Map<String, Value>, QueryError> {
        allocation::<(String, Value)>(length, self.location)?;
        self.admit(
            length
                .checked_add(1)
                .ok_or_else(|| failure(self.location, "JSON size overflow"))?,
        )?;
        Ok(Map::with_capacity(length))
    }

    fn string(&mut self, text: &str) -> Result<Value, QueryError> {
        self.admit(1)?;
        Ok(Value::String(self.copy_text(text)?))
    }

    fn copy_text(&mut self, text: &str) -> Result<String, QueryError> {
        allocation::<u8>(text.len(), self.location)?;
        self.admit(text.len())?;
        Ok(text.to_owned())
    }

    fn insert(
        &mut self,
        object: &mut Map<String, Value>,
        label: &str,
        value: Value,
    ) -> Result<(), QueryError> {
        object.insert(self.copy_text(label)?, value);
        Ok(())
    }

    fn tag(&mut self, kind: &str, fields: usize) -> Result<Map<String, Value>, QueryError> {
        let mut object = self.object(fields)?;
        let kind = self.string(kind)?;
        self.insert(&mut object, "kind", kind)?;
        Ok(object)
    }
}

fn visit_json(value: &Value, budget: &mut Budget, location: &Location) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match value {
            Value::Array(values) => {
                budget.charge(values.len(), location.clone())?;
                for value in values {
                    visit_json(value, budget, location)?;
                }
            }
            Value::Object(fields) => {
                budget.charge(fields.len(), location.clone())?;
                for (label, value) in fields {
                    budget.charge(label.len(), location.clone())?;
                    visit_json(value, budget, location)?;
                }
            }
            Value::String(text) => budget.charge(text.len(), location.clone())?,
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
        Ok(())
    })
}

struct OutputBuffer<'a> {
    bytes: Vec<u8>,
    budget: &'a mut Budget,
    location: &'a Location,
    error: Option<QueryError>,
}

impl OutputBuffer<'_> {
    fn append(&mut self, bytes: &[u8]) -> Result<(), QueryError> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| failure(self.location, "rendered output size overflow"))?;
        allocation::<u8>(length, self.location)?;
        self.budget.charge(bytes.len(), self.location.clone())?;
        self.budget
            .charge_results(bytes.len(), self.location.clone())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

impl Write for OutputBuffer<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Err(error) = self.append(bytes) {
            self.error = Some(error);
            return Err(io::Error::other("JSON output allowance exhausted"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn failure(location: &Location, message: impl Into<String>) -> QueryError {
    QueryError {
        location: location.clone(),
        message: message.into(),
        category: QueryErrorCategory::Other,
    }
}
