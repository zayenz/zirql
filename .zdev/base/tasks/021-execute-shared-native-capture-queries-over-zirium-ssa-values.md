+++
schema_version = 1
id = "base-021"
key = "native-capture-matching"
area = "base"
status = "open"
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

- [ ] from, bind and where obey ordered dependent scope and no-shadowing rules; native record field access feeds later shared project operations. Multiple from clauses are finite and budgeted before filtering.
- [ ] Zirium operand returns exact result/block-argument identity and same_value distinguishes values even when they share an owning operation.
- [ ] Example 3 returns only double with count 1 through inline/file/library paths. A short match expression with explicit json emission needs no file or named entry point.
- [ ] Every branch is validated for supported types/purity, empty inputs do not bypass validation, and runtime matching/copies share request work/result/nesting limits.

## Validation

- Use the example plus one same-owner/different-value counterexample; add a dependent-generator scope refusal and pre-filter work-limit control. Reuse existing native handle checks.
- Run required Zirql/Zirium checks and affected public API tests; document captures as native results rather than rendered identities.
