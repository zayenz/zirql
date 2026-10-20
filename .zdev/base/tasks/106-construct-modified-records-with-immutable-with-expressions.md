+++
schema_version = 1
id = "base-106"
key = "immutable-record-updates"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Construct modified records with immutable with expressions

## Outcome

Queries can reuse a record while replacing selected fields without mutating the original.

## Context

Record rebuilding is currently explicit. Start at src/value.rs, src/value/execution.rs, src/reader.rs and record type admission; docs/values.md owns field and missing-key behaviour. A what-if capacity report illustrates Zincite use without changing source data. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Defer field insertion/removal, general dictionary and keyed-array update syntax; record the keyed capacity-update idea as the next design question. No native source mutation or edit-policy changes.
- Keep this slice in Zirql and its local examples; record concrete consumer adoption requirements without changing consumer repositories.
- No Gecode dependency, symbolic CP lowering, first-class function system, publication or deployment.

## Done when

- [x] Textual record with {field: expression, ...} and its callable equivalent replace existing string fields only, preserve field order and leave the original value and retained aliases unchanged.
- [x] Evaluate the base once, then update expressions in written order in surrounding scope; updates do not create sequential field bindings. Missing or duplicate target fields fail. Result admission respects established record contracts.
- [x] An example reuses both original and updated capacity records. Focused coverage demonstrates order, alias preservation including native fields, and missing/duplicate/type refusal.
- [x] Update the relevant public language guide and callable/embedding documentation with the implemented contract, supported host/example entry point, limitations and a runnable example with expected output. Document necessary consumer adoption separately.

## Validation

- Use the focused behaviour checks described in Done when; run the documented local example and compare its result.
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- git diff --check
- zdev check base --format json

## Result

Implemented immutable textual/callable record updates with ordered evaluation, original field order, alias/native retention and explicit schema admission. Added capacity and API documentation. Independent verification passed without findings at Wf14bbf541f30a221.

Validation:

- Formatting, fatal workspace Clippy, workspace tests, diff whitespace and zdev consistency checks passed.
- Focused order/alias/native/missing/duplicate/schema controls passed; exact documented capacity output reproduced; final snapshot comparison equal.
