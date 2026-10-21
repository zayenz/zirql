use std::{cmp::Ordering, mem::size_of};

use crate::value::{
    admit_ordering_values,
    identity::{KeyRef, compare_refs},
};
use crate::{Budget, ConstructedValue, EvaluationError, Location, QueryError, QueryErrorCategory};

/// Native ordering retains host input/selector admission and original subjects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeOrder {
    Sort,
    Min,
    Max,
    MinAll,
    MaxAll,
}

/// A native stage with an optional singleton-relative query key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeOrdering<Q> {
    pub order: NativeOrder,
    pub selector: Option<Q>,
}

/// Owned host descriptors using the common structural value order.
/// Counts retain their unsigned range; JSON retains exact unsigned integers,
/// arrays and objects without conversion into signed constructed values.
#[derive(Debug, PartialEq)]
pub enum NativeOrderKey {
    Count(usize),
    Text(String),
    Value(ConstructedValue),
    Json(serde_json::Value),
}
impl NativeOrderKey {
    fn as_key(&self) -> KeyRef<'_> {
        match self {
            Self::Count(value) => KeyRef::Count(*value),
            Self::Text(value) => KeyRef::Text(value),
            Self::Value(value) => KeyRef::Value(value),
            Self::Json(value) => KeyRef::Json(value),
        }
    }
}

/// Order admitted native payloads without copying them or replacing their owner.
/// Keys are collected once in input order before any comparisons, so later key
/// failures remain observable for extrema too. Sort is stable; a single extreme
/// keeps its first tie and all-ties extremes keep input order. Empty input stays
/// empty after the host's normal preflight.
///
/// The callback uses this same budget for selector evaluation and admits any
/// text copy *before* allocating it. Moving an already-admitted owned text key
/// needs no second byte-copy charge. This operation admits temporary slots and
/// charges collection, descriptor writes, inspected comparison bytes and payload
/// moves, including with a legacy budget without an aggregate result counter.
///
/// # Errors
/// Propagates key failures and located item, work or temporary-storage refusals.
pub fn order_native_by<T, E>(
    mut items: Vec<T>,
    order: NativeOrder,
    budget: &mut Budget,
    location: Location,
    mut key: impl FnMut(&T, &mut Budget) -> Result<NativeOrderKey, EvaluationError<E>>,
) -> Result<Vec<T>, EvaluationError<E>> {
    let count = items.len();
    budget.check_items(count, location.clone())?;
    if count == 0 {
        return Ok(items);
    }
    admit_slots::<NativeOrderKey>(count, budget, &location)?;
    let mut keys = Vec::with_capacity(count);
    for item in &items {
        budget.charge(1, location.clone())?;
        let next = key(item, budget)?;
        keys.push(next);
    }
    admit_keys(&keys, budget, &location)?;
    if order == NativeOrder::Sort {
        let destinations = sort_indices(count, budget, &location, |left, right, budget| {
            compare(&keys[left], &keys[right], budget, &location)
        })?;
        apply_order(&mut items, destinations, budget, &location)?;
        return Ok(items);
    }
    let all = matches!(order, NativeOrder::MinAll | NativeOrder::MaxAll);
    let maximum = matches!(order, NativeOrder::Max | NativeOrder::MaxAll);
    admit_slots::<usize>(if all { count } else { 1 }, budget, &location)?;
    let mut selected = Vec::with_capacity(if all { count } else { 1 });
    selected.push(0);
    let mut best = 0;
    for index in 1..count {
        let comparison = compare(&keys[index], &keys[best], budget, &location)?;
        if comparison
            == if maximum {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        {
            best = index;
            selected.clear();
            budget.charge(1, location.clone())?;
            selected.push(index);
        } else if all && comparison == Ordering::Equal {
            budget.charge(1, location.clone())?;
            selected.push(index);
        }
    }
    budget.check_items(selected.len(), location.clone())?;
    budget.charge(count, location)?;
    let mut selected = selected.into_iter().peekable();
    let mut index = 0;
    items.retain(|_| {
        let keep = selected.peek() == Some(&index);
        if keep {
            selected.next();
        }
        index += 1;
        keep
    });
    Ok(items)
}

pub(crate) fn admit_keys(
    keys: &[NativeOrderKey],
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    admit_ordering_values(
        keys.iter().filter_map(|key| match key {
            NativeOrderKey::Value(value) => Some(value),
            _ => None,
        }),
        budget,
        location,
    )?;
    for key in keys {
        if let NativeOrderKey::Json(value) = key {
            admit_json(value, budget, location)?;
        }
    }
    Ok(())
}
fn admit_json(
    value: &serde_json::Value,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        match value {
            serde_json::Value::Number(value)
                if !value.is_i64()
                    && !value.is_u64()
                    && value.as_f64().is_none_or(|value| !value.is_finite()) =>
            {
                return Err(QueryError {
                    location: location.clone(),
                    message: "nonfinite float".into(),
                    category: QueryErrorCategory::Other,
                });
            }
            serde_json::Value::Array(values) => {
                budget.check_items(values.len(), location.clone())?;
                for value in values {
                    admit_json(value, budget, location)?;
                }
            }
            serde_json::Value::Object(values) => {
                budget.check_items(values.len(), location.clone())?;
                for value in values.values() {
                    admit_json(value, budget, location)?;
                }
            }
            _ => {}
        }
        Ok(())
    })
}

pub(crate) fn admit_slots<T>(
    count: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    let units = count
        .checked_add(1)
        .filter(|_| {
            count
                .checked_mul(size_of::<T>())
                .is_some_and(|bytes| bytes <= isize::MAX as usize)
        })
        .ok_or_else(|| QueryError {
            location: location.clone(),
            message: "native ordering storage limit exceeded".into(),
            category: QueryErrorCategory::ResultLimit,
        })?;
    budget.charge(units, location.clone())?;
    budget.charge_results(units, location.clone())
}

/// Return the destination of each source slot. Both native ordering and dictionary
/// sorting use this fallible stable merge, admitting its storage before work.
pub(crate) fn sort_indices(
    count: usize,
    budget: &mut Budget,
    location: &Location,
    mut compare: impl FnMut(usize, usize, &mut Budget) -> Result<Ordering, QueryError>,
) -> Result<Vec<usize>, QueryError> {
    admit_slots::<usize>(count, budget, location)?;
    let mut indices: Vec<usize> = (0..count).collect();
    admit_slots::<usize>(count, budget, location)?;
    let mut scratch = vec![0; count];
    let mut width = 1usize;
    while width < count {
        let mut start = 0;
        while start < count {
            let middle = start.saturating_add(width).min(count);
            let end = middle.saturating_add(width).min(count);
            let (mut left, mut right) = (start, middle);
            for slot in &mut scratch[start..end] {
                let take_left = right == end
                    || left < middle
                        && compare(indices[left], indices[right], budget)? != Ordering::Greater;
                budget.charge(1, location.clone())?;
                *slot = if take_left {
                    let index = indices[left];
                    left += 1;
                    index
                } else {
                    let index = indices[right];
                    right += 1;
                    index
                };
            }
            start = end;
        }
        std::mem::swap(&mut indices, &mut scratch);
        width = width.saturating_mul(2);
    }
    // Invert destination -> source before moving payloads in place.
    budget.charge(count, location.clone())?;
    for (destination, source) in indices.into_iter().enumerate() {
        scratch[source] = destination;
    }
    Ok(scratch)
}

pub(crate) fn apply_order<T>(
    items: &mut [T],
    mut destinations: Vec<usize>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    for index in 0..items.len() {
        while destinations[index] != index {
            budget.charge(1, location.clone())?;
            let destination = destinations[index];
            items.swap(index, destination);
            destinations.swap(index, destination);
        }
    }
    Ok(())
}

pub(crate) fn compare_text(
    left: &str,
    right: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    for index in 0..left.len().min(right.len()) {
        budget.charge(1, location.clone())?;
        let comparison = left.as_bytes()[index].cmp(&right.as_bytes()[index]);
        if comparison != Ordering::Equal {
            return Ok(comparison);
        }
    }
    Ok(left.len().cmp(&right.len()))
}

pub(crate) fn compare(
    left: &NativeOrderKey,
    right: &NativeOrderKey,
    budget: &mut Budget,
    location: &Location,
) -> Result<Ordering, QueryError> {
    compare_refs(left.as_key(), right.as_key(), budget, location)
}
