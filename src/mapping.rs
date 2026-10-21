use crate::construction::{EntryAccounting, construct_entries};

use crate::{Budget, EvaluationError, Location, QueryError, QueryErrorCategory};

/// Independent singleton-relative selectors for a string-keyed native map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMap<Q> {
    pub key: Q,
    pub value: Q,
}

/// Construct a map while retaining each host value without copying or rendering it.
/// Inputs run in their original order, key first, then value against that same
/// item. A key must yield exactly one string (`None` denotes a different shape).
/// A duplicate fails before its value runs. Entries retain insertion order.
///
/// Hosts preflight both selectors and input kinds even for empty input. The
/// callbacks use the active context and budget for singleton copies, selector
/// execution and host content limits. Already-admitted strings and values move
/// into the map. This operation admits the container and entry slots, checks
/// storage arithmetic and charges iteration and an upper bound on inspected key
/// bytes. These new work charges preserve the legacy program accounting policy.
///
/// # Errors
/// Propagates selector failures and located key, duplicate and resource refusals.
pub fn map_native_by<I, C: ?Sized, V, E>(
    items: Vec<I>,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
    mut key: impl FnMut(&I, &mut C, &mut Budget) -> Result<Option<Vec<String>>, EvaluationError<E>>,
    value: impl FnMut(I, &mut C, &mut Budget) -> Result<V, EvaluationError<E>>,
) -> Result<Vec<(String, V)>, EvaluationError<E>> {
    budget.check_items(items.len(), location.clone())?;
    construct_entries(
        items,
        context,
        budget,
        &location,
        |item, context, budget| {
            let Some(mut keys) = key(item, context, budget)? else {
                return Err(key_error(&location, budget)?.into());
            };
            if keys.len() != 1 {
                return Err(key_error(&location, budget)?.into());
            }
            Ok(keys.pop().expect("one map key"))
        },
        value,
        |key, _, budget| {
            let bytes = key
                .len()
                .checked_add("map_by encountered duplicate key ``; select unique keys".len())
                .ok_or_else(|| storage_error(&location))?;
            budget.charge_program_work(bytes, location.clone())?;
            budget.charge_results(bytes, location.clone())?;
            Ok(QueryError {
                location: location.clone(),
                message: format!("map_by encountered duplicate key `{key}`; select unique keys"),
                category: QueryErrorCategory::Other,
            })
        },
        EntryAccounting::NativeMap,
    )
}

fn key_error(location: &Location, budget: &mut Budget) -> Result<QueryError, QueryError> {
    let message = "map_by key must produce exactly one string";
    budget.charge_program_work(message.len(), location.clone())?;
    budget.charge_results(message.len(), location.clone())?;
    Ok(QueryError {
        location: location.clone(),
        message: message.into(),
        category: QueryErrorCategory::Other,
    })
}

fn storage_error(location: &Location) -> QueryError {
    QueryError {
        location: location.clone(),
        message: "native map storage limit exceeded".into(),
        category: QueryErrorCategory::ResultLimit,
    }
}

/// Sort already-admitted unique-key entries without copying their payloads.
/// Collect and admit all descriptors before sorting in the common structural
/// value order. Equal values retain their incoming entry order.
///
/// The callback shares this budget for host access and any text copy.
/// This operation admits descriptor and index storage, charges descriptor
/// visits, comparisons (including inspected scalar text bytes), and payload moves.
/// Legacy budgets charge work too. Payload ownership and rendering stay with
/// the caller; no serialization is needed.
///
/// # Errors
/// Propagates projection failures and located value, item, work and storage
/// refusals, including invalid descriptors on singleton inputs.
pub fn sort_map_by_value<V, E>(
    mut entries: Vec<(String, V)>,
    budget: &mut Budget,
    location: Location,
    mut key: impl FnMut(&V, &mut Budget) -> Result<crate::NativeOrderKey, EvaluationError<E>>,
) -> Result<Vec<(String, V)>, EvaluationError<E>> {
    use crate::ordering::{admit_slots, apply_order, sort_indices};

    let count = entries.len();
    budget.check_items(count, location.clone())?;
    if count == 0 {
        return Ok(entries);
    }
    admit_slots::<crate::NativeOrderKey>(count, budget, &location)?;
    let mut keys = Vec::with_capacity(count);
    for (_, value) in &entries {
        budget.charge(1, location.clone())?;
        keys.push(key(value, budget)?);
    }
    crate::ordering::admit_keys(&keys, budget, &location)?;
    let destinations = sort_indices(count, budget, &location, |left, right, budget| {
        crate::ordering::compare(&keys[left], &keys[right], budget, &location)
    })?;
    apply_order(&mut entries, destinations, budget, &location)?;
    Ok(entries)
}

/// Select dictionary entries, retaining their current order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapSelection {
    Reverse,
    Head(usize),
    Tail(usize),
}

/// Move or drop admitted entries without copying payloads. Reverse includes
/// tied keys. Head/tail accept zero and oversized bounds. Charge entry work
/// before mutating the vector; no new result storage is allocated.
///
/// # Errors
/// Returns a located item or work refusal without changing the entries.
pub fn select_map<V>(
    entries: &mut Vec<(String, V)>,
    selection: MapSelection,
    budget: &mut Budget,
    location: Location,
) -> Result<(), QueryError> {
    let count = entries.len();
    budget.check_items(count, location.clone())?;
    let work = match selection {
        MapSelection::Reverse => count,
        MapSelection::Head(keep) => count.saturating_sub(keep),
        MapSelection::Tail(keep) if keep < count => count,
        MapSelection::Tail(_) => 0,
    };
    budget.charge(work, location)?;
    match selection {
        MapSelection::Reverse => entries.reverse(),
        MapSelection::Head(keep) => entries.truncate(keep),
        MapSelection::Tail(keep) => crate::tail(entries, keep),
    }
    Ok(())
}
