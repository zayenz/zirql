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

/// A host-projected scalar used to rank map entries. Numeric variants share
/// one category and compare exactly; the other categories must not mix.
/// Hosts reject nested payloads and admit copied text before constructing it.
#[derive(Debug, PartialEq)]
pub enum ScalarMapKey {
    Null,
    Boolean(bool),
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    String(String),
}

/// Rank already-admitted unique-key entries without copying their payloads.
/// Collect and validate descriptors before sorting. Values must be all null,
/// all Boolean, all numeric or all string; floats must be finite. Equal values
/// use lexical entry keys, regardless of the incoming entry order.
///
/// The callback shares this budget for host access and any text copy. The
/// caller admits conversion from a map into entry slots before allocation.
/// This operation admits descriptor and index storage, charges descriptor
/// visits, comparisons (including inspected text/key bytes), and payload moves.
/// Legacy budgets charge work too. Payload ownership and rendering stay with
/// the caller; no serialization is needed.
///
/// # Errors
/// Propagates projection failures and located scalar, item, work and storage
/// refusals, including invalid descriptors on singleton inputs.
pub fn rank_map_by_value<V, E>(
    mut entries: Vec<(String, V)>,
    budget: &mut Budget,
    location: Location,
    mut key: impl FnMut(&V, &mut Budget) -> Result<ScalarMapKey, EvaluationError<E>>,
) -> Result<Vec<(String, V)>, EvaluationError<E>> {
    use crate::ordering::{admit_slots, apply_order, compare_text, sort_indices};

    let count = entries.len();
    budget.check_items(count, location.clone())?;
    if count == 0 {
        return Ok(entries);
    }
    admit_slots::<ScalarMapKey>(count, budget, &location)?;
    let mut keys = Vec::with_capacity(count);
    let mut category = None;
    for (_, value) in &entries {
        budget.charge(1, location.clone())?;
        let key = key(value, budget)?;
        let current = scalar_category(&key, budget, &location)?;
        if category.is_some_and(|category| category != current) {
            return Err(scalar_error(
                location,
                "sort_by(value) requires comparable scalar map values of one type",
                budget,
            )?
            .into());
        }
        category = Some(current);
        keys.push(key);
    }
    let destinations = sort_indices(count, budget, &location, |left, right, budget| {
        let comparison = compare_scalars(&keys[left], &keys[right], budget, &location)?;
        if comparison == std::cmp::Ordering::Equal {
            compare_text(&entries[left].0, &entries[right].0, budget, &location)
        } else {
            Ok(comparison)
        }
    })?;
    apply_order(&mut entries, destinations, budget, &location)?;
    Ok(entries)
}

/// Select entries of an explicitly ranked map, retaining their current order.
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
pub fn select_ranked_map<V>(
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScalarCategory {
    Null,
    Boolean,
    Number,
    String,
}

fn scalar_category(
    key: &ScalarMapKey,
    budget: &mut Budget,
    location: &Location,
) -> Result<ScalarCategory, QueryError> {
    Ok(match key {
        ScalarMapKey::Null => ScalarCategory::Null,
        ScalarMapKey::Boolean(_) => ScalarCategory::Boolean,
        ScalarMapKey::Integer(_) | ScalarMapKey::Unsigned(_) => ScalarCategory::Number,
        ScalarMapKey::Float(value) if value.is_finite() => ScalarCategory::Number,
        ScalarMapKey::Float(_) => {
            return Err(scalar_error(
                location.clone(),
                "sort_by(value) requires finite numeric map values",
                budget,
            )?);
        }
        ScalarMapKey::String(_) => ScalarCategory::String,
    })
}

fn compare_scalars(
    left: &ScalarMapKey,
    right: &ScalarMapKey,
    budget: &mut Budget,
    location: &Location,
) -> Result<std::cmp::Ordering, QueryError> {
    use ScalarMapKey as K;
    use std::cmp::Ordering;

    budget.charge(1, location.clone())?;
    Ok(match (left, right) {
        (K::Null, K::Null) => Ordering::Equal,
        (K::Boolean(left), K::Boolean(right)) => left.cmp(right),
        (K::String(left), K::String(right)) => {
            crate::ordering::compare_text(left, right, budget, location)?
        }
        (K::Float(left), K::Float(right)) => left.partial_cmp(right).expect("finite scalar keys"),
        (K::Float(left), right) => compare_integer_float(integer(right), *left).reverse(),
        (left, K::Float(right)) => compare_integer_float(integer(left), *right),
        (left, right) => integer(left).cmp(&integer(right)),
    })
}

fn integer(key: &ScalarMapKey) -> i128 {
    match key {
        ScalarMapKey::Integer(value) => i128::from(*value),
        ScalarMapKey::Unsigned(value) => i128::from(*value),
        _ => unreachable!("map descriptors have one admitted scalar category"),
    }
}

fn compare_integer_float(integer: i128, float: f64) -> std::cmp::Ordering {
    // Integers fit in i64/u64. Saturation of this wider cast still orders
    // out-of-range floats correctly; compare fractional parts only on a tie.
    integer.cmp(&(float as i128)).then_with(|| {
        0.0f64
            .partial_cmp(&float.fract())
            .expect("finite scalar key")
    })
}

fn scalar_error(
    location: Location,
    message: &str,
    budget: &mut Budget,
) -> Result<QueryError, QueryError> {
    budget.charge(message.len(), location.clone())?;
    budget.charge_results(message.len(), location.clone())?;
    Ok(QueryError {
        location,
        message: message.into(),
        category: QueryErrorCategory::Other,
    })
}
