+++
schema_version = 1
id = "base-009"
key = "zirium-advanced-hosts"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-004"]
+++
# Complete Zirium diff, fixed-point and live-edit integration

## Outcome

Zirium's advanced queries execute shared language structure while retaining side-scoped diff identity, bounded traversal and established live-edit behavior.

## Context

Read Zirium query/diff.rs, diff builders, semantic-diff docs and existing diff_query/query tests. Ordinary expressions and programs already use Zirql. Extend the host operation boundary only for concrete nested fixed-point, diff-stream and effect needs; keep domain algorithms local. Use the shared runtime context for nested bodies, bound results and emitters.

## Boundaries

- No shared MLIR graph, dialect registry or semantic diff algorithm in Zirql.
- Preserve ordered fixed-point convergence/cycle behavior, before/after side identity, stale-handle checks and diff edit refusal.
- Retain Zirium's existing edit atomicity and library emission contract; do not promise whole-program edit rollback.

## Done when

- [ ] Fixed-point and advanced navigation stages invoke nested evaluation without resetting work/item/nesting bounds.
- [ ] Text and typed diff builders preserve change/operation-side result types, projections and same-side operations through shared execution.
- [ ] Live edits remain visible through retained native handles while saved scalar results retain their values; invalidated handles and diff edits are refused.
- [ ] Explicit/implicit emission order, pass-through emitters and CLI later-error buffering remain compatible.
- [ ] Redundant shared statement/pipeline implementations are removed after all advanced paths use the core, with domain traversal and mutation still host-owned.

## Validation

- Reuse current fixed-point, diff and edit/emission tests; add focused controls only for an uncovered integration boundary.
- Run required Zirql/Zirium checks and relevant Python diff/query tests; no new graph benchmark campaign.
