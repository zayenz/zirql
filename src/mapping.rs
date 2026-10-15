use std::{collections::BTreeMap, collections::btree_map::Entry, mem::size_of};

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
/// A duplicate fails before its value runs. Returned keys have lexical order.
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
    mut value: impl FnMut(I, &mut C, &mut Budget) -> Result<V, EvaluationError<E>>,
) -> Result<BTreeMap<String, V>, EvaluationError<E>> {
    budget.check_items(items.len(), location.clone())?;
    budget.charge_program_work(1, location.clone())?;
    budget.charge_results(1, location.clone())?;
    let mut entries = BTreeMap::new();
    for item in items {
        budget.charge_program_work(1, location.clone())?;
        let Some(mut keys) = key(&item, context, budget)? else {
            return Err(key_error(&location, budget)?.into());
        };
        if keys.len() != 1 {
            return Err(key_error(&location, budget)?.into());
        }
        let key = keys.pop().expect("one map key");
        let count = entries
            .len()
            .checked_add(1)
            .ok_or_else(|| storage_error(&location))?;
        count
            .checked_mul(size_of::<(String, V)>())
            .filter(|bytes| *bytes <= isize::MAX as usize)
            .ok_or_else(|| storage_error(&location))?;
        // One tree descent inspects each existing key at most once. Charging
        // the incoming key length bounds each comparison's inspected bytes.
        let inspected = key
            .len()
            .checked_mul(count)
            .ok_or_else(|| storage_error(&location))?;
        budget.charge_program_work(inspected, location.clone())?;
        match entries.entry(key) {
            Entry::Occupied(entry) => {
                let bytes = entry
                    .key()
                    .len()
                    .checked_add("map_by encountered duplicate key ``; select unique keys".len())
                    .ok_or_else(|| storage_error(&location))?;
                budget.charge_program_work(bytes, location.clone())?;
                budget.charge_results(bytes, location.clone())?;
                return Err(QueryError {
                    location,
                    message: format!(
                        "map_by encountered duplicate key `{}`; select unique keys",
                        entry.key()
                    ),
                    category: QueryErrorCategory::Other,
                }
                .into());
            }
            Entry::Vacant(entry) => {
                let mapped = value(item, context, budget)?;
                budget.check_items(count, location.clone())?;
                budget.charge_program_work(1, location.clone())?;
                budget.charge_results(1, location.clone())?;
                entry.insert(mapped);
            }
        }
    }
    Ok(entries)
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
