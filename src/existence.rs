use crate::{Budget, EvaluationError, Location};

/// Keep original native items whose singleton-relative relation is nonempty.
/// Relation results are inspected independently in input order; retained items
/// preserve their order, duplicates and native handles without copying payloads.
///
/// The host preflights the relation before calling this operation, including for
/// empty input. The callback evaluates the relation against the borrowed item,
/// using the supplied active budget for singleton input, native work and result
/// admission. Outer bindings and function argument scopes stay with that runner.
/// This operation charges iteration and admits each retained result before storing it.
///
/// # Errors
/// Propagates every relation failure, including after earlier successful matches,
/// and located work, item and result limit refusals. A failure never means false.
pub fn filter_native_exists<T, R, E>(
    items: Vec<T>,
    budget: &mut Budget,
    location: Location,
    mut relation: impl FnMut(&T, &mut Budget) -> Result<Vec<R>, EvaluationError<E>>,
) -> Result<Vec<T>, EvaluationError<E>> {
    budget.check_items(items.len(), location.clone())?;
    let mut selected = Vec::new();
    for item in items {
        budget.charge(1, location.clone())?;
        if !relation(&item, budget)?.is_empty() {
            budget.charge(1, location.clone())?;
            budget.charge_results(1, location.clone())?;
            selected.push(item);
        }
    }
    Ok(selected)
}
