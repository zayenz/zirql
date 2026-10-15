use std::{borrow::Cow, collections::BTreeMap};

use zirql::{
    Budget, EvaluationError, Location, ProgramLimits, QueryError, QueryErrorCategory,
    construct_string_entries, expand_fragments,
};

fn limits() -> ProgramLimits {
    ProgramLimits {
        work: 10_000,
        results: 10_000,
        stream_items: 16,
        nesting: 8,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Captioned<'a> {
    native: &'a str,
    caption: String,
}

fn construct<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str, bool)>,
    budget: &mut Budget,
) -> Result<BTreeMap<String, Captioned<'a>>, EvaluationError<&'static str>> {
    let location = Location::Query(10..30);
    construct_string_entries(
        rows,
        &mut "row:",
        budget,
        location.clone(),
        |(name, _, _), prefix, budget| {
            expand_fragments(
                [*prefix, name],
                prefix,
                budget,
                location.clone(),
                |text, _, _| Ok(Cow::Borrowed(text)),
            )
        },
        |(name, native, fails), prefix, budget| {
            if fails {
                return Err(EvaluationError::Host("value failed"));
            }
            let caption = expand_fragments(
                [name, " caption"],
                prefix,
                budget,
                location.clone(),
                |text, _, _| Ok(Cow::Borrowed(text)),
            )?;
            Ok(Captioned { native, caption })
        },
        |key, _, budget| {
            let bytes = "duplicate ".len() + key.len();
            budget.charge(bytes, location.clone())?;
            budget.charge_results(bytes, location.clone())?;
            Ok(QueryError::query(10..30, format!("duplicate {key}")))
        },
    )
}

#[test]
fn fragments_compose_with_entries_and_retain_native_values_in_written_failure_order() {
    let source = String::from("beta alpha");
    let entries = construct(
        [("b", &source[..4], false), ("a", &source[5..], false)],
        &mut Budget::for_program(limits()),
    )
    .unwrap();
    assert_eq!(
        entries.keys().map(String::as_str).collect::<Vec<_>>(),
        ["row:a", "row:b"]
    );
    assert_eq!(entries["row:b"].caption, "b caption");
    assert_eq!(entries["row:b"].native.as_ptr(), source.as_ptr());
    assert_eq!(entries["row:a"].native, "alpha");

    let error = construct(
        [("a", "first", false), ("a", "duplicate", true)],
        &mut Budget::for_program(limits()),
    )
    .unwrap_err();
    assert!(matches!(error, EvaluationError::Core(error) if error.message == "duplicate row:a"));
    let error = construct(
        [
            ("z", "first", true),
            ("a", "second", false),
            ("a", "third", false),
        ],
        &mut Budget::for_program(limits()),
    )
    .unwrap_err();
    assert_eq!(error, EvaluationError::Host("value failed"));
}

#[test]
fn construction_refusals_keep_locations_and_restore_active_nesting_without_refunds() {
    let location = Location::Query(40..55);
    let mut budget = Budget::for_program(ProgramLimits {
        results: 3,
        ..limits()
    });
    let error = expand_fragments(
        ["four"],
        &mut (),
        &mut budget,
        location.clone(),
        |text, _, _| Ok::<_, EvaluationError<()>>(Cow::Borrowed(text)),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.category == QueryErrorCategory::ResultLimit && error.location == location)
    );
    assert_eq!(budget.remaining_results(), Some(2));
    assert!(budget.remaining_work() < limits().work);

    let mut budget = Budget::for_program(ProgramLimits {
        work: 1,
        ..limits()
    });
    let error = expand_fragments(
        ["x"],
        &mut (),
        &mut budget,
        location.clone(),
        |text, _, _| Ok::<_, EvaluationError<()>>(Cow::Borrowed(text)),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.category == QueryErrorCategory::WorkLimit && error.location == location)
    );

    let mut budget = Budget::for_program(ProgramLimits {
        nesting: 1,
        ..limits()
    });
    let error = budget
        .within_evaluation(1, location.clone(), |budget| {
            construct([("a", "native", false)], budget)
        })
        .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.message == "nesting limit exceeded" && error.location == Location::Query(10..30))
    );
    assert!(construct([("a", "native", false)], &mut budget).is_ok());
    let error = budget
        .within_evaluation::<(), QueryError>(1, location.clone(), |budget| {
            budget.within_evaluation(usize::MAX, location.clone(), |_| Ok(()))
        })
        .unwrap_err();
    assert_eq!(error.message, "nesting limit exceeded");
    assert_eq!(error.location, location);
}
