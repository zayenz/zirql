use std::{borrow::Cow, mem::size_of};

use crate::value::identity::{HashIndex, hash_string};
use crate::{Budget, EvaluationError, Location, QueryError, QueryErrorCategory};

/// Resolve and append text fragments once, in written order.
///
/// The resolver borrows existing text or admits any owned temporary before
/// allocating it. It uses the supplied context and active budget for binding
/// admission and host conversions. This operation charges each visit, copied
/// byte and produced byte before growing its private output. It checks byte
/// arithmetic and bounds callback reentry without adding a logical value level.
/// Parsing, escaping and print newlines belong to the caller; inserted text is
/// never parsed again. No partial string is returned on failure.
///
/// # Errors
/// Propagates resolver failures and located work, result, storage and reentry
/// refusals. Resolver errors retain their own locations.
pub fn expand_fragments<'a, P, C: ?Sized, E>(
    parts: impl IntoIterator<Item = P>,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
    mut resolve: impl FnMut(P, &mut C, &mut Budget) -> Result<Cow<'a, str>, EvaluationError<E>>,
) -> Result<String, EvaluationError<E>> {
    budget.within_evaluation(0, location.clone(), |budget| {
        budget.charge_results(1, location.clone())?;
        let mut output = String::new();
        for part in parts {
            budget.charge(1, location.clone())?;
            let text = resolve(part, context, budget)?;
            output
                .len()
                .checked_add(text.len())
                .filter(|bytes| *bytes <= isize::MAX as usize)
                .ok_or_else(|| storage_error(&location, "fragment storage limit exceeded"))?;
            budget.charge(text.len(), location.clone())?;
            budget.charge_results(text.len(), location.clone())?;
            output.push_str(&text);
        }
        Ok(output)
    })
}

/// Construct computed string-key entries, retaining arbitrary admitted values.
///
/// Each entry runs its key, checks for a collision, then runs its value before
/// proceeding to the next entry. A duplicate fails before that value runs. The
/// diagnostic callback admits and builds the caller's duplicate error; it cannot
/// allow a duplicate. Entries retain insertion order. All partial data stays
/// private on failure.
///
/// Callbacks share the active context and budget. Admit key/value allocations
/// before returning them; already-admitted payloads move into the map without a
/// copy charge. This operation enters one container level, admits container and
/// entry slots, checks storage arithmetic and charges hashing, collision checks
/// and inspected key bytes. Do not wrap it in another guard for that container.
///
/// # Errors
/// Propagates callback failures and located duplicate, item, work, result,
/// nesting and storage refusals. Child errors retain their own locations.
pub fn construct_string_entries<I, C: ?Sized, V, E>(
    entries: impl IntoIterator<Item = I>,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
    key: impl FnMut(&I, &mut C, &mut Budget) -> Result<String, EvaluationError<E>>,
    value: impl FnMut(I, &mut C, &mut Budget) -> Result<V, EvaluationError<E>>,
    duplicate: impl FnMut(&str, &mut C, &mut Budget) -> Result<QueryError, EvaluationError<E>>,
) -> Result<Vec<(String, V)>, EvaluationError<E>> {
    budget.within_evaluation(1, location.clone(), |budget| {
        construct_entries(
            entries,
            context,
            budget,
            &location,
            key,
            value,
            duplicate,
            EntryAccounting::Computed,
        )
    })
}

pub(crate) enum EntryAccounting {
    Computed,
    NativeMap,
}

impl EntryAccounting {
    fn charge(
        &self,
        budget: &mut Budget,
        units: usize,
        location: &Location,
    ) -> Result<(), QueryError> {
        match self {
            Self::Computed => budget.charge(units, location.clone()),
            Self::NativeMap => budget.charge_program_work(units, location.clone()),
        }
    }

    pub(crate) fn entries<V>(
        &self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<(String, V)>, QueryError> {
        self.charge(budget, 1, location)?;
        budget.charge_results(1, location.clone())?;
        Ok(Vec::new())
    }

    pub(crate) fn contains<V>(
        &self,
        entries: &[(String, V)],
        candidates: &[usize],
        key: &str,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<bool, QueryError> {
        let count = entries
            .len()
            .checked_add(1)
            .ok_or_else(|| self.storage_error(location))?;
        count
            .checked_mul(size_of::<(String, V)>())
            .filter(|bytes| *bytes <= isize::MAX as usize)
            .ok_or_else(|| self.storage_error(location))?;
        for &index in candidates {
            let previous = &entries[index].0;
            let inspected = previous
                .len()
                .min(key.len())
                .checked_add(1)
                .ok_or_else(|| self.storage_error(location))?;
            self.charge(budget, inspected, location)?;
            if previous == key {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn append<V>(
        &self,
        entries: &mut Vec<(String, V)>,
        key: String,
        value: V,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        let count = entries
            .len()
            .checked_add(1)
            .ok_or_else(|| self.storage_error(location))?;
        budget.check_items(count, location.clone())?;
        self.charge(budget, 1, location)?;
        budget.charge_results(1, location.clone())?;
        entries.push((key, value));
        Ok(())
    }

    fn storage_error(&self, location: &Location) -> QueryError {
        storage_error(
            location,
            match self {
                Self::Computed => "object construction storage limit exceeded",
                Self::NativeMap => "native map storage limit exceeded",
            },
        )
    }
}

// Native maps already have an enclosing expression guard. Keep their legacy
// work policy while sharing the same key/check/value insertion mechanism.
#[expect(
    clippy::too_many_arguments,
    reason = "The shared loop retains separate key, value and diagnostic callbacks and the existing native-map accounting policy."
)]
pub(crate) fn construct_entries<I, C: ?Sized, V, E>(
    entries: impl IntoIterator<Item = I>,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
    mut key: impl FnMut(&I, &mut C, &mut Budget) -> Result<String, EvaluationError<E>>,
    mut value: impl FnMut(I, &mut C, &mut Budget) -> Result<V, EvaluationError<E>>,
    mut duplicate: impl FnMut(&str, &mut C, &mut Budget) -> Result<QueryError, EvaluationError<E>>,
    accounting: EntryAccounting,
) -> Result<Vec<(String, V)>, EvaluationError<E>> {
    let mut result = accounting.entries(budget, location)?;
    let mut hashes = HashIndex::default();
    for item in entries {
        accounting.charge(budget, 1, location)?;
        let key = key(&item, context, budget)?;
        accounting.charge(
            budget,
            key.len()
                .checked_add(1)
                .ok_or_else(|| accounting.storage_error(location))?,
            location,
        )?;
        let hash = hash_string(&key);
        if accounting.contains(&result, hashes.candidates(hash), &key, budget, location)? {
            return Err(duplicate(&key, context, budget)?.into());
        }
        let mapped = value(item, context, budget)?;
        let position = result.len();
        accounting.append(&mut result, key, mapped, budget, location)?;
        match &accounting {
            EntryAccounting::Computed => hashes.insert(hash, position, budget, location)?,
            EntryAccounting::NativeMap => {
                hashes.insert_program(hash, position, budget, location)?
            }
        }
    }
    Ok(result)
}

fn storage_error(location: &Location, message: &str) -> QueryError {
    QueryError {
        location: location.clone(),
        message: message.into(),
        category: QueryErrorCategory::ResultLimit,
    }
}
