use std::{collections::HashSet, hash::Hash, mem::size_of};

use crate::{Budget, EvaluationError, Location, QueryError, QueryErrorCategory};

/// Native set operators have equal precedence and associate left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeSetOperator {
    Union,
    Intersect,
    Except,
}

impl NativeSetOperator {
    /// Read an exact, case-sensitive native set keyword.
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "union" => Some(Self::Union),
            "intersect" => Some(Self::Intersect),
            "except" => Some(Self::Except),
            _ => None,
        }
    }
}

/// Combine native identities in the supplied candidate order, keeping each once.
/// The host first validates both operand kinds and every handle, including empty
/// or discarded operands. Equality and hashing must agree on native identity.
/// Candidates must cover every possible selected identity in the required order;
/// they may repeat identities or include identities absent from both operands.
///
/// This operation admits membership storage before allocation and charges each
/// insertion, candidate visit, membership probe and output append. The callback
/// charges additional identity work, such as inspected string bytes, before each
/// insertion or probe. Fixed-size identities may return `Ok(())`. Only selected
/// output receives the stream-item ceiling; scanning canonical candidates does
/// not impose that ceiling on the host document. Payloads are moved, not cloned.
/// Additional host scans and owned output copies use this same active budget.
///
/// # Errors
/// Propagates candidate/identity failures and located work, result or item limits.
pub fn combine_native_set<T: Eq + Hash, E>(
    operator: NativeSetOperator,
    left: impl ExactSizeIterator<Item = T>,
    right: impl ExactSizeIterator<Item = T>,
    ordered_candidates: impl IntoIterator<Item = Result<T, EvaluationError<E>>>,
    budget: &mut Budget,
    location: Location,
    mut identity_work: impl FnMut(&T, &mut Budget) -> Result<(), EvaluationError<E>>,
) -> Result<Vec<T>, EvaluationError<E>> {
    let mut left = membership(left, budget, &location, &mut identity_work)?;
    let mut right = membership(right, budget, &location, &mut identity_work)?;
    admit_slots::<T>(0, budget, &location)?;
    let mut selected = Vec::new();
    for candidate in ordered_candidates {
        budget.charge(1, location.clone())?;
        let candidate = candidate?;
        budget.charge(1, location.clone())?;
        identity_work(&candidate, budget)?;
        let in_left = left.remove(&candidate);
        budget.charge(1, location.clone())?;
        identity_work(&candidate, budget)?;
        let in_right = right.remove(&candidate);
        let keep = match operator {
            NativeSetOperator::Union => in_left || in_right,
            NativeSetOperator::Intersect => in_left && in_right,
            NativeSetOperator::Except => in_left && !in_right,
        };
        if keep {
            let count = selected
                .len()
                .checked_add(1)
                .ok_or_else(|| storage_error(&location))?;
            budget.check_items(count, location.clone())?;
            check_storage::<T>(count, &location)?;
            budget.charge(1, location.clone())?;
            budget.charge_results(1, location.clone())?;
            selected
                .try_reserve(1)
                .map_err(|_| storage_error(&location))?;
            selected.push(candidate);
        }
    }
    Ok(selected)
}

fn membership<T: Eq + Hash, E>(
    items: impl ExactSizeIterator<Item = T>,
    budget: &mut Budget,
    location: &Location,
    identity_work: &mut impl FnMut(&T, &mut Budget) -> Result<(), EvaluationError<E>>,
) -> Result<HashSet<T>, EvaluationError<E>> {
    admit_slots::<T>(items.len(), budget, location)?;
    let mut membership = HashSet::new();
    membership
        .try_reserve(items.len())
        .map_err(|_| storage_error(location))?;
    for item in items {
        budget.charge(1, location.clone())?;
        identity_work(&item, budget)?;
        membership.insert(item);
    }
    Ok(membership)
}

fn admit_slots<T>(
    count: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    check_storage::<T>(count, location)?;
    let units = count
        .checked_add(1)
        .ok_or_else(|| storage_error(location))?;
    budget.charge(units, location.clone())?;
    budget.charge_results(units, location.clone())
}

fn check_storage<T>(count: usize, location: &Location) -> Result<(), QueryError> {
    count
        .checked_mul(size_of::<T>().max(1))
        .filter(|bytes| *bytes <= isize::MAX as usize)
        .map(|_| ())
        .ok_or_else(|| storage_error(location))
}

fn storage_error(location: &Location) -> QueryError {
    QueryError {
        location: location.clone(),
        message: "native set storage limit exceeded".into(),
        category: QueryErrorCategory::ResultLimit,
    }
}
