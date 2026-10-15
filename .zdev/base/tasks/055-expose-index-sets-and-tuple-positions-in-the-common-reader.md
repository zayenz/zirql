+++
schema_version = 1
id = "base-055"
key = "complete-common-collection-reader"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Expose index sets and tuple positions in the common reader

## Outcome

Common value expressions can use index_set(array) and one-based tuple access through the existing shared evaluator, including Zirium integration.

## Context

The host-contract audit found that src/reader.rs omits index_set from its builtin calls and treats every dot postfix as a record field. Existing ValueExpressionKind::IndexSet and TupleAccess already validate and execute these operations in src/value.rs and src/value/execution.rs; Zincite lowers them in crates/zincite-query/src/comprehension.rs. In the words example index_set(words) currently becomes an unresolved named call, and (1, 2).1 fails parsing. Read the base brief and its linked comprehension contract. Base-047 owns constructed pipeline composition; this task only exposes the existing collection operations.

## Boundaries

- Reuse existing value representations, validation and execution. Preserve record field access, existing numeric/range grammar, host native queries and shared one-based positions. No new set/keyed constructors, evaluator, Zincite grammar or data-expression policy.

## Done when

- [ ] The common reader lowers index_set(array) and tuple.1 (and other positive positions) to the existing shared nodes; chained tuple/record/array access composes without changing their established precedence.
- [ ] Positional arrays and rank-one keyed arrays retain existing index-set identity, order and typed-empty behavior; wrong kinds/ranks and invalid tuple positions produce located errors through existing preflight/runtime rules and the active budget.
- [ ] The words example and Zirium common-expression entry points exercise the forms. Update the affected Zirium pinned dependency when needed and document the common spelling; existing Zincite lowering remains compatible.

## Validation

- Add only focused common-reader behavior coverage: index_set on positional and host-supplied keyed input, tuple access composed with a field, and representative rank/position/type failures. Reuse existing evaluator tests and demonstrate one Zirium inline/library control.
- Run the brief-required Zirql Rust checks, affected Zirium checks and existing query controls, git diff --check and zdev check base --format json. Record separate consumer commits; no new broad test matrix.
