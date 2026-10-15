+++
schema_version = 1
id = "base-073"
key = "csv-runtime-header-schema"
area = "base"
status = "open"
complexity = "standard"
afk = true
blocked_by = ["base-070"]
+++
# Retain CSV header schemas in runtime rows

## Outcome

Computed CSV lookup and membership enforce the same declared header schema as literal column access, while valid queries retain their results and bounded execution.

## Context

The CSV adapter declares header record types in examples/csv.rs::Table::types, but decode_table stores plain records and read_binding/read_current_input copy those unannotated values. On the people fixture, let column = "missing"; lookup(rows[1], column) returns null and has_key(rows[1], column) returns false, although literal missing labels fail. docs/csv.md and the brief require labels outside a declared schema to fail. Read existing ConstructedValue::Declared and declaration propagation through child access and optional_access; core already implements the required distinction for annotated runtime values. This defect predates the brace-reader task and needs no new lookup semantics.

## Boundaries

- Keep this a CSV adapter repair using existing declaration, copy, admission and rendering APIs. Do not add another evaluator, schema framework, per-row declaration copies, type guessing, numeric conversions or new core value kinds.
- Preserve exact UTF-8 header/cell strings, named and implicit input selection, aggregate source-byte cap, one active budget and buffered JSON. No consumer changes, dependency updates, publication or deployment.

## Done when

- [ ] Runtime CSV table values retain their header declaration through named/implicit reads, saved bindings, row selection and existing copies. Valid computed lookup and has_key work; unknown computed labels fail like literal labels, with original query locations and empty failed-request stdout. Keep preflight header checks for empty tables.
- [ ] Existing JSON output and insertion order are unchanged, including nested row output and one/multiple named tables. Declaration storage, admission and copying remain charged through the current budget.
- [ ] Extend one existing focused CSV example control for valid and invalid computed labels, saved-row reuse and the documented schema distinction. Reuse existing input/budget/refusal controls and exact guide examples; no syntax matrix, timing assertions or new test harness. Update guide wording only if necessary to describe the actual fixed contract.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative checkout, required Rust formatting/Clippy/locked workspace tests and Rust 1.88 tests. Run the CSV example tests explicitly if workspace tests do not exercise examples.
- Run the existing CSV guide commands and compact computed-label success/refusal cases against the actual example, checking buffered output and order. For changed guides run site check/build/link checks and desktop/mobile review; documentation needs no new product tests.
