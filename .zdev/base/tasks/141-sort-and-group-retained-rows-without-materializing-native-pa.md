+++
schema_version = 1
id = "base-141"
key = "native-row-ordering"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Sort and group retained rows without materializing native payloads

## Outcome

Shared sort_by and group_by read ordinary row keys while retaining unrelated native payloads.

## Context

A report sequence of {"key":"same","subject":n} renders in Zirium but sort_by(rows,"key") and group_by(rows,"key") force whole-row materialization and fail. Core functions.rs compute_collection lacks retained row handling and falls back to value/execution.rs apply, which materializes operands. Native stage sort_by("key") treats the argument as a constant selector and is not an equivalent fix.

## Boundaries

- Reuse current ordering and grouping algorithms and views. Do not add a comparator framework, JSON roundtrip or iterable protocol.

## Done when

- [x] Common field-key sort_by and group_by operate on rows containing native payloads through existing container views and host constructors.
- [x] Sorting is stable, grouping retains first-seen key/group order, and row values, native owners and declared empty schemas are preserved.
- [x] Key admission and budgets use existing common rules. Reading ordinary keys does not require projecting whole native payloads.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Add a focused public retained-row sort/group control with distinct keys and a tie; reuse key-admission and budget controls.
- Verify the ordinary constructed equivalent and the affected native report path.

## Result

Common sort_by and group_by read ordinary row keys while retaining native payloads. Sorting preserves the order of tied rows, grouping preserves first-seen order, and empty results retain their declared schemas.

Validation:

- Required offline locked Rust formatting, Clippy and tests passed in affected repositories; Zirium doctests and warnings-denied docs passed. Focused public controls passed.
- Astra medium reviewed added prose using unslop and zayenz-writing-style. Both consumers use canonical Zirql 4c4f5625f75dea8d0195da95d136b5992f6a5dcf.
- Independent verifier passed; exact core snapshot and consumer states matched before completion.
