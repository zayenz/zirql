+++
schema_version = 1
id = "base-018"
key = "scalar-summaries"
area = "base"
status = "done"
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

- [x] length, checked integer sum, forall and exists work across validation, free-name discovery, type discovery, evaluation and copying as applicable; empty identities and all charges are documented.
- [x] require accepts a literal string message and preflights every operand, but evaluates its value only after a true known-Boolean condition; false, arithmetic failures and limit exhaustion retain locations and fail the request.
- [x] Both Zincite commands accept the brief's --program TEXT form through the existing Program API. Example 2 runs with a local literal factor until typed parameters land; it reports totals 18/19 with only index 3 overloaded.
- [x] A one-line literal-demand sum and its file/library equivalent agree. Help and documentation distinguish new supported expressions from model execution.

## Validation

- Reuse example 2 for success; add focused empty reduction, sum overflow, eager-head-error and skipped-require-value controls plus one later-error stdout check.
- Run required Zirql/Zincite checks and applicable CLI tests. Preserve separate consumer commits and pinned Git integration; no publication.

## Result

Added shared eager length, checked integer sum, forall/exists and literal-message require; Zincite uses them through inline/file/library programs while retaining its data expression subset. Core fadce51b6d8fc65bc12458dc57f3d9b2092c2cb5 and consumer 2b7450d224e1c5bb8712827410cf3f1448685a12 are separate verified commits.

Validation:

- Independent whole-task PASS at W59ea800b7a566e92, exact root comparison equal and fourteen consumer source/index hashes unchanged; all four completion conditions verified.
- Both canonical repositories: cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace and git diff --check passed; consumer cached diff check and core zdev check passed.
- Both documentation sites npm check/build passed; guide links/rendering inspected. Fresh public CLI captures prove totals 18/19, only index 3 overloaded, short sum 9 inline/file/library agreement and later-error exit 2 with empty stdout. No publication.
