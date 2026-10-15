+++
schema_version = 1
id = "base-049"
key = "shared-native-ordering"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Share native selector ordering and all-ties extrema

## Outcome

Zirql supplies bounded native sorting and extrema, including nested query keys and retention of every tied extreme, and Zirium adopts them.

## Context

Zirium query.rs::sort_operations_by/extreme_value and query/diff_program.rs::order plus Sort/Extreme dispatch own native ordering. Zirql src/value.rs::sort_value and Extremum already cover constructed arrays and static record fields; base-036 is done and base-047 exposes constructed sorting in pipes. The gap is reusable native scalar/selector ordering and all-ties extrema. Read query-language.md Ordering, bounds, and extrema and query-dsl.md Nested queries and native maps.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [ ] Shared typed construction and common native reading support sort, sort_by(query), min/max, min_by/max_by(query), min_all/max_all and min_all_by/max_all_by(query). Selectors receive each singleton input independently and produce one string or count; prepared function arguments and saved bindings retain their scopes.
- [ ] Stable sorting retains equal-key input order. Single extrema retain the first tie; all-ties extrema retain every equal-key item in input order. Compute selector keys once per item, sort in O(n log n) comparisons and select extrema in O(n) comparisons without sorting the whole input.
- [ ] Ordinary and typed Zirium extrema still refuse empty input; diff extrema still return empty selections; existing constructed extrema remain optional. Preserve native Count/Text key ordering, supported kinds and purity restrictions without imposing constructed-value rules on legacy native queries.
- [ ] Zirium ordinary/typed/diff paths use the shared algorithms and retain native handles, side checks, key/type errors, locations and one active budget. Key collection, comparisons and temporary storage are admitted and charged.

## Validation

- Reuse existing ordering controls for duplicate keys, first/all ties, ordinary-versus-diff empty behavior, wrong-cardinality keys and nested selector failures. Add only a compact core native control proving reuse outside Zirium and budget refusal; do not assert internal callback counts or timings.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.
