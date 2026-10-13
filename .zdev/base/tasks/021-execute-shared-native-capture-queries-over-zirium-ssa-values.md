+++
schema_version = 1
id = "base-021"
key = "native-capture-matching"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-019"]
+++
# Execute shared native capture queries over Zirium SSA values

## Outcome

Zirql owns finite native capture composition and Zirium demonstrates the same-SSA-value structural query from the CLI and library.

## Context

Read example 3, shared generator rules and src/program.rs native execution. Reuse scope/enumeration machinery where it fits without admitting native handles to ConstructedValue. Zirium navigation currently returns defining operations rather than exact SSA operand values.

## Boundaries

- Implement common match parsing, located representation, validation, native capture records and execution in Zirql. Zirium supplies typed operand and same_value primitives.
- Required captures only; an unavailable required operand or Unknown guard gives a located refusal. Preserve incomplete-source reporting, handle lifetimes and document/side identity. No snippet rewriting or variadic patterns.

## Done when

- [x] from, bind and where obey ordered dependent scope and no-shadowing rules; native record field access feeds later shared project operations. Multiple from clauses are finite and budgeted before filtering.
- [x] Zirium operand returns exact result/block-argument identity and same_value distinguishes values even when they share an owning operation.
- [x] Example 3 returns only double with count 1 through inline/file/library paths. A short match expression with explicit json emission needs no file or named entry point.
- [x] Every branch is validated for supported types/purity, empty inputs do not bypass validation, and runtime matching/copies share request work/result/nesting limits.

## Validation

- Use the example plus one same-owner/different-value counterexample; add a dependent-generator scope refusal and pre-filter work-limit control. Reuse existing native handle checks.
- Run required Zirql/Zirium checks and affected public API tests; document captures as native results rather than rendered identities.

## Result

Shared finite native captures and Zirium exact SSA queries are complete. Ordered from/bind/where and native fields feed shared projection with full scope/type/purity preflight and continuing budgets. Example 3 returns only double with count 1 through inline, file and library paths; a short match | json works directly. Core API 6fef2436a311b6f83379d129d3000db0b494b314 and Zirium integration 94de33bb894ce8e3d46841fbfbb246810c47bf39 are committed and pushed separately. Zincite remains unchanged.

Validation:

- Fresh independent whole-task PASS at Wf6002e7edc97a361, zero corrections; all four conditions checked and exact three-repository/source state preserved.
- All 37 independently run required checks passed: core fmt/Clippy/tests/doctests/diff/zdev; Zirium Rust 1.88/stable all-feature tests/Clippy, stable fmt/doctests/warning-free docs/diff; actual extension rebuild then Ruff/ty/full pytest for CPython 3.11–3.14 (168 passed and 2 existing macOS /dev/full skips each); both site check/build.
- Fresh seven changed guides, 334 local links/anchors and canonical /zirql/ and /zirium/ paths passed. Original fixture bytes and immutable Git pin are preserved.
- GitHub Quality run 38463513041 passed all 12 jobs for Zirium commit 94de33b: https://github.com/zayenz/zirium/actions/runs/38463513041.
