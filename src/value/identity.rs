use std::cmp::Ordering;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::Hasher;

use super::{
    ConstructedValue as V, ValueKey, ValueKeyType, check_member_pair, constructed_value_type,
};
use crate::{Budget, Location, QueryError};

pub(crate) const ARRAY: u8 = 6;
pub(crate) const TUPLE: u8 = 7;
pub(crate) const SET: u8 = 8;
pub(crate) const DICTIONARY: u8 = 9;
pub(crate) const ENTRY: u8 = 11;

/// Hash structural language identity, ignoring collection insertion order and declarations.
/// The hash is suitable for request-local indexes, not persistent identifiers.
///
/// # Errors
/// Invalid data and exhausted work or nesting allowances retain `location`.
pub fn constructed_value_hash(
    value: &V,
    budget: &mut Budget,
    location: Location,
) -> Result<u64, QueryError> {
    constructed_value_type(value, budget, location.clone())?;
    hash_admitted(value, budget, &location)
}

/// Compare all constructed value kinds without scalar coercion. Sets and
/// dictionaries compare canonical membership/associations; arrays retain order.
///
/// # Errors
/// Invalid data, contradictory nominal metadata and exhausted allowances fail.
pub fn compare_constructed_values(
    left: &V,
    right: &V,
    budget: &mut Budget,
    location: Location,
) -> Result<Ordering, QueryError> {
    constructed_value_type(left, budget, location.clone())?;
    constructed_value_type(right, budget, location.clone())?;
    compare_admitted(left, right, budget, &location)
}

pub(crate) fn hash_parts(tag: u8, parts: impl IntoIterator<Item = u64>) -> u64 {
    let mut state = DefaultHasher::new();
    state.write_u8(tag);
    let mut count = 0u64;
    for part in parts {
        state.write_u64(part);
        count = count.wrapping_add(1);
    }
    state.write_u64(count);
    state.finish()
}

pub(crate) fn hash_unordered(tag: u8, parts: impl IntoIterator<Item = u64>) -> u64 {
    let (mut sum, mut xor, mut count) = (0u64, 0u64, 0u64);
    for part in parts {
        let mixed = hash_parts(255, [part]);
        sum = sum.wrapping_add(mixed);
        xor ^= mixed.rotate_left((mixed & 63) as u32);
        count = count.wrapping_add(1);
    }
    hash_parts(tag, [count, sum, xor])
}

pub(crate) fn hash_string(value: &str) -> u64 {
    let mut state = DefaultHasher::new();
    state.write_u8(4);
    state.write_usize(value.len());
    state.write(value.as_bytes());
    state.finish()
}

pub(crate) fn hash_admitted(
    value: &V,
    budget: &mut Budget,
    location: &Location,
) -> Result<u64, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match value.data() {
            V::Absent => hash_parts(0, []),
            V::Boolean(value) => hash_parts(1, [u64::from(*value)]),
            V::Integer(value) => hash_parts(2, [*value as u64]),
            V::Float(value) => hash_parts(3, [if *value == 0.0 { 0 } else { value.to_bits() }]),
            V::String(value) => {
                budget.charge(value.len(), location.clone())?;
                hash_string(value)
            }
            V::Member(value) => hash_parts(5, [value.type_id.0, value.member_id.0]),
            V::Array(_) | V::Tuple(_) | V::Set { .. } => {
                let values = super::collection_elements(value.data());
                let mut hashes = super::metadata_vec(values.len(), budget, location)?;
                for value in values {
                    hashes.push(hash_admitted(value, budget, location)?);
                }
                match value.data() {
                    V::Array(_) => hash_parts(ARRAY, hashes),
                    V::Tuple(_) => hash_parts(TUPLE, hashes),
                    _ => hash_unordered(SET, hashes),
                }
            }
            V::Record(fields) => {
                let mut hashes = super::metadata_vec(fields.len(), budget, location)?;
                for (key, value) in fields {
                    budget.charge(key.len(), location.clone())?;
                    hashes.push(hash_parts(
                        ENTRY,
                        [hash_string(key), hash_admitted(value, budget, location)?],
                    ));
                }
                hash_unordered(DICTIONARY, hashes)
            }
            V::Dictionary(entries) => {
                let mut hashes = super::metadata_vec(entries.len(), budget, location)?;
                for (key, value) in entries {
                    hashes.push(hash_parts(
                        ENTRY,
                        [
                            hash_admitted(key, budget, location)?,
                            hash_admitted(value, budget, location)?,
                        ],
                    ));
                }
                hash_unordered(DICTIONARY, hashes)
            }
            V::KeyedArray { axes, entries } => {
                let mut axis_hashes = super::metadata_vec(axes.len(), budget, location)?;
                for axis in axes {
                    let mut keys = super::metadata_vec(axis.keys.len() + 1, budget, location)?;
                    keys.push(match axis.key_type {
                        ValueKeyType::Integer => hash_parts(12, []),
                        ValueKeyType::Member(id) => hash_parts(13, [id.0]),
                    });
                    for key in &axis.keys {
                        keys.push(hash_key(key, budget, location)?);
                    }
                    axis_hashes.push(hash_parts(14, keys));
                }
                let mut entry_hashes = super::metadata_vec(entries.len(), budget, location)?;
                for entry in entries {
                    let mut keys = super::metadata_vec(entry.key.len(), budget, location)?;
                    for key in &entry.key {
                        keys.push(hash_key(key, budget, location)?);
                    }
                    entry_hashes.push(hash_parts(
                        ENTRY,
                        [
                            hash_parts(TUPLE, keys),
                            hash_admitted(&entry.value, budget, location)?,
                        ],
                    ));
                }
                hash_parts(
                    10,
                    [hash_parts(15, axis_hashes), hash_parts(16, entry_hashes)],
                )
            }
            V::Declared { .. } => unreachable!("data unwraps declarations"),
        })
    })
}

fn hash_key(key: &ValueKey, budget: &mut Budget, location: &Location) -> Result<u64, QueryError> {
    budget.charge(1, location.clone())?;
    Ok(match key {
        ValueKey::Integer(value) => hash_parts(2, [*value as u64]),
        ValueKey::Member(value) => hash_parts(5, [value.type_id.0, value.member_id.0]),
    })
}

/// Collision buckets preserve insertion positions; callers resolve collisions by equality.
#[derive(Default)]
pub(crate) struct HashIndex(HashMap<u64, Vec<usize>>);
impl HashIndex {
    pub(crate) fn candidates(&self, hash: u64) -> &[usize] {
        self.0.get(&hash).map_or(&[], Vec::as_slice)
    }
    pub(crate) fn insert(
        &mut self,
        hash: u64,
        index: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        budget.charge(2, location.clone())?;
        self.insert_slot(hash, index, location)
    }

    pub(crate) fn insert_program(
        &mut self,
        hash: u64,
        index: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        budget.charge_program_work(2, location.clone())?;
        self.insert_slot(hash, index, location)
    }

    fn insert_slot(
        &mut self,
        hash: u64,
        index: usize,
        location: &Location,
    ) -> Result<(), QueryError> {
        if !self.0.contains_key(&hash) {
            self.0
                .try_reserve(1)
                .map_err(|_| super::error(location, "dictionary index allocation failed"))?;
        }
        let bucket = self.0.entry(hash).or_default();
        bucket
            .try_reserve(1)
            .map_err(|_| super::error(location, "dictionary index allocation failed"))?;
        bucket.push(index);
        Ok(())
    }
}

/// Borrow existing key storage without building another recursive value tree.
#[derive(Clone, Copy)]
pub(crate) enum KeyRef<'a> {
    Value(&'a V),
    Text(&'a str),
    Count(usize),
    Json(&'a serde_json::Value),
}
impl<'a> KeyRef<'a> {
    fn data(self) -> Self {
        match self {
            Self::Value(value) => Self::Value(value.data()),
            other => other,
        }
    }
    fn rank(self) -> u8 {
        match self.data() {
            Self::Value(value) => rank(value),
            Self::Text(_) => 4,
            Self::Count(_) => 2,
            Self::Json(value) => match value {
                serde_json::Value::Null => 0,
                serde_json::Value::Bool(_) => 1,
                serde_json::Value::Number(value) if value.is_i64() || value.is_u64() => 2,
                serde_json::Value::Number(_) => 3,
                serde_json::Value::String(_) => 4,
                serde_json::Value::Array(_) => ARRAY,
                serde_json::Value::Object(_) => DICTIONARY,
            },
        }
    }
    fn boolean(self) -> bool {
        match self.data() {
            Self::Value(V::Boolean(value)) | Self::Json(serde_json::Value::Bool(value)) => *value,
            _ => unreachable!(),
        }
    }
    fn integer(self) -> i128 {
        match self.data() {
            Self::Value(V::Integer(value)) => i128::from(*value),
            Self::Count(value) => value as i128,
            Self::Json(serde_json::Value::Number(value)) => value
                .as_i64()
                .map(i128::from)
                .unwrap_or_else(|| i128::from(value.as_u64().expect("integer JSON key"))),
            _ => unreachable!(),
        }
    }
    fn float(self) -> f64 {
        match self.data() {
            Self::Value(V::Float(value)) => *value,
            Self::Json(serde_json::Value::Number(value)) => value.as_f64().expect("float JSON key"),
            _ => unreachable!(),
        }
    }
    fn text(self) -> &'a str {
        match self.data() {
            Self::Text(value) => value,
            Self::Value(V::String(value)) | Self::Json(serde_json::Value::String(value)) => value,
            _ => unreachable!(),
        }
    }
    fn len(self) -> usize {
        match self.data() {
            Self::Value(V::Array(values) | V::Tuple(values)) => values.len(),
            Self::Json(serde_json::Value::Array(values)) => values.len(),
            _ => unreachable!(),
        }
    }
    fn at(self, index: usize) -> Self {
        match self.data() {
            Self::Value(V::Array(values) | V::Tuple(values)) => Self::Value(&values[index]),
            Self::Json(serde_json::Value::Array(values)) => Self::Json(&values[index]),
            _ => unreachable!(),
        }
    }
}
pub(crate) fn rank(value: &V) -> u8 {
    match value.data() {
        V::Absent => 0,
        V::Boolean(_) => 1,
        V::Integer(_) => 2,
        V::Float(_) => 3,
        V::String(_) => 4,
        V::Member(_) => 5,
        V::Array(_) => ARRAY,
        V::Tuple(_) => TUPLE,
        V::Set { .. } => SET,
        V::Record(_) | V::Dictionary(_) => DICTIONARY,
        V::KeyedArray { .. } => 10,
        V::Declared { .. } => unreachable!("data unwraps declarations"),
    }
}

pub(crate) fn compare_admitted(
    left: &V,
    right: &V,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    compare_refs(KeyRef::Value(left), KeyRef::Value(right), budget, location)
}

pub(crate) fn compare_refs(
    left: KeyRef<'_>,
    right: KeyRef<'_>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        let kind = left.rank();
        let order = kind.cmp(&right.rank());
        if order != Ordering::Equal {
            return Ok(order);
        }
        match kind {
            0 => return Ok(Ordering::Equal),
            1 => return Ok(left.boolean().cmp(&right.boolean())),
            2 => return Ok(left.integer().cmp(&right.integer())),
            3 => {
                let (a, b) = (left.float(), right.float());
                return Ok(if a == b {
                    Ordering::Equal
                } else {
                    a.total_cmp(&b)
                });
            }
            4 => return compare_text(left.text(), right.text(), budget, location),
            ARRAY | TUPLE => return compare_sequence(left, right, budget, location),
            DICTIONARY => {
                let a = sorted_entries(left, budget, location)?;
                let b = sorted_entries(right, budget, location)?;
                for ((ak, av), (bk, bv)) in a.iter().zip(&b) {
                    let mut order = compare_refs(*ak, *bk, budget, location)?;
                    if order == Ordering::Equal {
                        order = compare_refs(*av, *bv, budget, location)?;
                    }
                    if order != Ordering::Equal {
                        return Ok(order);
                    }
                }
                return Ok(a.len().cmp(&b.len()));
            }
            _ => {}
        }
        let (KeyRef::Value(left), KeyRef::Value(right)) = (left.data(), right.data()) else {
            unreachable!("constructed-only kind")
        };
        Ok(match (left, right) {
            (V::Member(a), V::Member(b)) => {
                check_member_pair(a, b, budget, location)?;
                (a.type_id.0, a.declaration_order).cmp(&(b.type_id.0, b.declaration_order))
            }
            (V::Set { elements: a, .. }, V::Set { elements: b, .. }) => {
                let a = sorted_values(a, budget, location)?;
                let b = sorted_values(b, budget, location)?;
                let mut order = Ordering::Equal;
                for (a, b) in a.iter().zip(&b) {
                    order = compare_admitted(a, b, budget, location)?;
                    if order != Ordering::Equal {
                        break;
                    }
                }
                order.then(a.len().cmp(&b.len()))
            }
            (
                V::KeyedArray {
                    axes: a,
                    entries: ae,
                },
                V::KeyedArray {
                    axes: b,
                    entries: be,
                },
            ) => {
                let mut order = Ordering::Equal;
                for (a, b) in a.iter().zip(b) {
                    budget.charge(1, location.clone())?;
                    let ty = |ty| match ty {
                        ValueKeyType::Integer => (0, 0),
                        ValueKeyType::Member(id) => (1, id.0),
                    };
                    order = ty(a.key_type).cmp(&ty(b.key_type));
                    if order == Ordering::Equal {
                        order = compare_keys(&a.keys, &b.keys, budget, location)?;
                    }
                    if order != Ordering::Equal {
                        break;
                    }
                }
                order = order.then(a.len().cmp(&b.len()));
                if order == Ordering::Equal {
                    for (a, b) in ae.iter().zip(be) {
                        order = compare_keys(&a.key, &b.key, budget, location)?;
                        if order == Ordering::Equal {
                            order = compare_admitted(&a.value, &b.value, budget, location)?;
                        }
                        if order != Ordering::Equal {
                            break;
                        }
                    }
                    order = order.then(ae.len().cmp(&be.len()));
                }
                order
            }
            _ => unreachable!("equal ranks have matching representations"),
        })
    })
}
fn compare_text(
    a: &str,
    b: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    budget.charge(a.len().min(b.len()), location.clone())?;
    Ok(a.cmp(b))
}
fn compare_sequence(
    a: KeyRef<'_>,
    b: KeyRef<'_>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    for index in 0..a.len().min(b.len()) {
        let order = compare_refs(a.at(index), b.at(index), budget, location)?;
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(a.len().cmp(&b.len()))
}
fn compare_keys(
    a: &[ValueKey],
    b: &[ValueKey],
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    for (a, b) in a.iter().zip(b) {
        budget.charge(1, location.clone())?;
        let order = match (a, b) {
            (ValueKey::Integer(a), ValueKey::Integer(b)) => a.cmp(b),
            (ValueKey::Member(a), ValueKey::Member(b)) => {
                check_member_pair(a, b, budget, location)?;
                (a.type_id.0, a.declaration_order).cmp(&(b.type_id.0, b.declaration_order))
            }
            (ValueKey::Integer(_), ValueKey::Member(_)) => Ordering::Less,
            _ => Ordering::Greater,
        };
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(a.len().cmp(&b.len()))
}
fn sorted_values<'a>(
    values: &'a [V],
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<&'a V>, QueryError> {
    let mut sorted = super::metadata_vec(values.len(), budget, location)?;
    sorted.extend(values);
    sort(&mut sorted, budget, location, |a, b, budget| {
        compare_admitted(a, b, budget, location)
    })?;
    Ok(sorted)
}
fn sorted_entries<'a>(
    value: KeyRef<'a>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<(KeyRef<'a>, KeyRef<'a>)>, QueryError> {
    let len = match value.data() {
        KeyRef::Value(V::Record(v)) => v.len(),
        KeyRef::Value(V::Dictionary(v)) => v.len(),
        KeyRef::Json(serde_json::Value::Object(v)) => v.len(),
        _ => unreachable!(),
    };
    let mut sorted = super::metadata_vec(len, budget, location)?;
    match value.data() {
        KeyRef::Value(V::Record(fields)) => sorted.extend(
            fields
                .iter()
                .map(|(k, v)| (KeyRef::Text(k), KeyRef::Value(v))),
        ),
        KeyRef::Value(V::Dictionary(entries)) => sorted.extend(
            entries
                .iter()
                .map(|(k, v)| (KeyRef::Value(k), KeyRef::Value(v))),
        ),
        KeyRef::Json(serde_json::Value::Object(entries)) => sorted.extend(
            entries
                .iter()
                .map(|(k, v)| (KeyRef::Text(k), KeyRef::Json(v))),
        ),
        _ => unreachable!(),
    }
    sort(&mut sorted, budget, location, |a, b, budget| {
        compare_refs(a.0, b.0, budget, location)
    })?;
    Ok(sorted)
}

// Fallible bottom-up merge sort keeps budget errors outside a comparison callback.
pub(crate) fn sort<T: Copy, E: From<QueryError>>(
    values: &mut [T],
    budget: &mut Budget,
    location: &Location,
    mut compare: impl FnMut(&T, &T, &mut Budget) -> Result<Ordering, E>,
) -> Result<(), E> {
    let mut scratch = super::metadata_vec(values.len(), budget, location)?;
    scratch.extend_from_slice(values);
    let mut width = 1;
    while width < values.len() {
        for start in (0..values.len()).step_by(width.saturating_mul(2)) {
            let mid = (start + width).min(values.len());
            let end = start
                .saturating_add(width.saturating_mul(2))
                .min(values.len());
            let (mut left, mut right) = (start, mid);
            for target in &mut scratch[start..end] {
                budget.charge(1, location.clone())?;
                if right == end
                    || left < mid
                        && compare(&values[left], &values[right], budget)? != Ordering::Greater
                {
                    *target = values[left];
                    left += 1;
                } else {
                    *target = values[right];
                    right += 1;
                }
            }
        }
        values.copy_from_slice(&scratch);
        width = width.saturating_mul(2);
    }
    Ok(())
}
