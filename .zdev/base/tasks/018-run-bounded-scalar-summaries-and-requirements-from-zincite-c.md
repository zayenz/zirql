+++
schema_version = 1
id = "base-018"
key = "scalar-summaries"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-005"]
+++
# Run bounded scalar summaries and requirements from Zincite command lines

## Outcome

Zirql evaluates small checked reducers and pure requirements; Zincite exposes them through inline, file and library programs.

## Context

Read the brief's Proposed extension bundle and example 2 in [language examples](../background/language-design-examples.md). Start with src/value.rs and tests/value_expressions.rs, then Zincite's program lowering, query CLI and umbrella query dispatch. ValueExpressionKind currently has no reducers; Zincite program input currently requires a file.

## Boundaries

- Change Zirql and the two Zincite query entry points within this read-only reporting slice. Preserve existing pipelines, tagged results and completeness.
- Use closed base operations, eager materialized arrays and conditional require; no arbitrary calls, float sum, lazy generator fusion or edits.

## Done when

- [ ] length, checked integer sum, forall and exists work across validation, free-name discovery, type discovery, evaluation and copying as applicable; empty identities and all charges are documented.
- [ ] require accepts a literal string message and preflights every operand, but evaluates its value only after a true known-Boolean condition; false, arithmetic failures and limit exhaustion retain locations and fail the request.
- [ ] Both Zincite commands accept the brief's --program TEXT form through the existing Program API. Example 2 runs with a local literal factor until typed parameters land; it reports totals 18/19 with only index 3 overloaded.
- [ ] A one-line literal-demand sum and its file/library equivalent agree. Help and documentation distinguish new supported expressions from model execution.

## Validation

- Reuse example 2 for success; add focused empty reduction, sum overflow, eager-head-error and skipped-require-value controls plus one later-error stdout check.
- Run required Zirql/Zincite checks and applicable CLI tests. Preserve separate consumer commits and pinned Git integration; no publication.
