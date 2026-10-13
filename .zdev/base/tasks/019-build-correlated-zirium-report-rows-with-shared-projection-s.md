+++
schema_version = 1
id = "base-019"
key = "correlated-projection"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-018"]
+++
# Build correlated Zirium report rows with shared projection syntax

## Outcome

The base language projects native items into ordered constructed records, and Zirium runs the function report as a short command or saved program.

## Context

Read example 1 and the brief's base/domain ownership rules. Inspect src/program.rs native records/reentry, src/parser.rs common reader, and Zirium query parser, builder and literal/comprehension lowering. Existing map_by requires unique string keys; literal fields cannot contain pipelines.

## Boundaries

- Zirql owns new projection/value syntax support, subject scope, purity, cardinality and evaluation. Supply a reusable common reader for the new forms and typed construction APIs; Zirium contributes domain parsing/lowering hooks only.
- Preserve existing Zirium syntax and native APIs, including bindings and emitters. No JSON transport of native handles or unrelated parser rewrite.

## Done when

- [ ] project(subject, {fields}) evaluates fields in order against each bound native subject, preserves row order/duplicates, and constructs native Zirql report data with explicit scalar projections.
- [ ] Shared one extraction rejects zero/multiple projected values; count conversion checks the shared integer range. Empty inputs still receive scope/type/purity validation; nested work retains one budget.
- [ ] Zirium exposes the base reducers/require and new projection through text and callable APIs. Example 1 with literal-bound arguments produces the documented two rows and totals 5/2.
- [ ] The documented short CLI projection runs without a file, declaration block or named entry point. Common let/report forms in the full example and legacy host emission remain compatible; file/library outputs agree.

## Validation

- Use the function fixture with an ordering/duplicate-name control, one-cardinality refusal and a nested budget/purity refusal. Reuse existing program and builder suites rather than a syntax matrix.
- Run required Zirql/Zirium checks and affected Python checks when that binding surface changes; document native results and inline usage.
