use crate::{Budget, EvaluationError, Location, QueryError, QueryErrorCategory};

/// Repeatedly replace a native selection until its ordered contents are unchanged.
/// The final body execution runs before convergence is checked, including its
/// host-approved emissions. A repeated non-fixed selection is an error. Brent's
/// cycle policy retains one checkpoint rather than a history of selections.
///
/// All callbacks receive the same mutable host context and active budget. `next`
/// evaluates the retained body on an admitted copy of the current selection;
/// `copy` admits checkpoint traversal, payload work and storage before copying;
/// `same` checks native kinds/owners and ordered, duplicate-sensitive equality.
/// The host keeps body preparation, native admission and nested evaluation guards.
/// Iterations and comparisons must also be charged with legacy budgets.
///
/// # Errors
/// Propagates body, copy and comparison failures. Cycles and counter overflow
/// fail at `location`; budget failures retain their original locations.
pub fn evaluate_native_fixed_point<V, C, E>(
    mut current: V,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
    mut next: impl FnMut(&V, &mut C, &mut Budget) -> Result<V, EvaluationError<E>>,
    mut copy: impl FnMut(&V, &mut C, &mut Budget) -> Result<V, EvaluationError<E>>,
    mut same: impl FnMut(&V, &V, &mut C, &mut Budget) -> Result<bool, EvaluationError<E>>,
) -> Result<V, EvaluationError<E>> {
    let mut checkpoint = copy(&current, context, budget)?;
    let mut power = 1usize;
    let mut distance = 0usize;
    loop {
        let following = next(&current, context, budget)?;
        budget.charge(1, location.clone())?;
        if same(&following, &current, context, budget)? {
            return Ok(current);
        }
        if same(&following, &checkpoint, context, budget)? {
            return Err(failure(
                &location,
                "fixpoint query cycles without reaching an unchanged selection",
            )
            .into());
        }
        current = following;
        distance = distance
            .checked_add(1)
            .ok_or_else(|| failure(&location, "fixpoint iteration counter overflow"))?;
        if distance == power {
            checkpoint = copy(&current, context, budget)?;
            power = power
                .checked_mul(2)
                .ok_or_else(|| failure(&location, "fixpoint iteration counter overflow"))?;
            distance = 0;
        }
    }
}

/// Compare native selections positionally, preserving order and duplicates.
/// Hosts first check native kind and owner/side, including for empty selections.
/// The callback supplies primitive identity/equality and charges variable-sized
/// inspection before doing it. This operation charges length and item visits.
///
/// # Errors
/// Propagates located item/work limits and primitive comparison failures.
pub fn ordered_native_equal<T, E>(
    left: &[T],
    right: &[T],
    budget: &mut Budget,
    location: Location,
    mut same_item: impl FnMut(&T, &T, &mut Budget) -> Result<bool, EvaluationError<E>>,
) -> Result<bool, EvaluationError<E>> {
    budget.check_items(left.len(), location.clone())?;
    budget.check_items(right.len(), location.clone())?;
    budget.charge(1, location.clone())?;
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left, right) in left.iter().zip(right) {
        budget.charge(1, location.clone())?;
        if !same_item(left, right, budget)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn failure(location: &Location, message: &str) -> QueryError {
    QueryError {
        location: location.clone(),
        message: message.into(),
        category: QueryErrorCategory::Other,
    }
}
