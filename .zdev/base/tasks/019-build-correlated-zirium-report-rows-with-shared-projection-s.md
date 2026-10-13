+++
schema_version = 1
id = "base-019"
key = "correlated-projection"
area = "base"
status = "done"
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

- [x] project(subject, {fields}) evaluates fields in order against each bound native subject, preserves row order/duplicates, and constructs native Zirql report data with explicit scalar projections.
- [x] Shared one extraction rejects zero/multiple projected values; count conversion checks the shared integer range. Empty inputs still receive scope/type/purity validation; nested work retains one budget.
- [x] Zirium exposes the base reducers/require and new projection through text and callable APIs. Example 1 with literal-bound arguments produces the documented two rows and totals 5/2.
- [x] The documented short CLI projection runs without a file, declaration block or named entry point. Common let/report forms in the full example and legacy host emission remain compatible; file/library outputs agree.

## Validation

- Use the function fixture with an ordering/duplicate-name control, one-cardinality refusal and a nested budget/purity refusal. Reuse existing program and builder suites rather than a syntax matrix.
- Run required Zirql/Zirium checks and affected Python checks when that binding surface changes; document native results and inline usage.

## Result

Shared ordered native projection and query-aware value reading/evaluation now power Zirium text, Rust and Python reports. Literal-bound full and short inline/file/library reports agree with totals 5/2; legacy names, literals and emission remain compatible. Core 02d54c6e5d1b99b434eed75bf9640888a7fa9dd2; Zirium f1e4a4ad8daeb22d4ab875b45aa3eebe8241b62a.

Validation:

- Independent whole-task PASS at W7415136a0f08f164; exact frozen source and final completion comparison equal. Both source commits pushed separately.
- Core fmt, all-targets workspace Clippy with warnings denied, workspace tests and 3 doctests, diff and zdev checks pass.
- Actual Git-pin Zirium Rust 1.88/stable full all-target/all-feature workspace tests and Clippy, doctests and warning-free API docs pass.
- Real rebuilt CPython 3.11 through 3.14 each pass format, lint, type and full tests: 166 passed, 2 unchanged macOS /dev/full skips.
- Native row ordering/duplicates, explicit scalars, shared one/count, empty scope/type/purity, nested budget and lazy require controls pass; actual CLI local binding collision returns [1], and late failure produces no stdout.
- Both site check/build pass; seven affected generated guides and 369 internal links/fragments resolve, with local projection guide visual review. No new publication or prerelease.
