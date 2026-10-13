+++
schema_version = 1
id = "base-009"
key = "zirium-advanced-hosts"
area = "base"
status = "done"
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

- [x] Fixed-point and advanced navigation stages invoke nested evaluation without resetting work/item/nesting bounds.
- [x] Text and typed diff builders preserve change/operation-side result types, projections and same-side operations through shared execution.
- [x] Live edits remain visible through retained native handles while saved scalar results retain their values; invalidated handles and diff edits are refused.
- [x] Explicit/implicit emission order, pass-through emitters and CLI later-error buffering remain compatible.
- [x] Redundant shared statement/pipeline implementations are removed after all advanced paths use the core, with domain traversal and mutation still host-owned.

## Validation

- Reuse current fixed-point, diff and edit/emission tests; add focused controls only for an uncovered integration boundary.
- Run required Zirql/Zirium checks and relevant Python diff/query tests; no new graph benchmark campaign.

## Result

Zirium text and typed diff queries execute shared language structure with native side identity and one nested budget, preserving live edits, emission policies and buffered CLI output. Long flat expressions are disposed iteratively through the additive ownership seam. Core source 56e561c; consumer source 29cbc66, pinned to that exact Git revision.

Validation:

- Independent whole-task PASS at W4079d70dbdcec48c; all five completion conditions covered and exact coordinator comparison passed before lifecycle mutation. All 18 consumer source hashes match reviewed bytes after the separate source commit.
- All 33 required checks pass: Zirql fmt, workspace Clippy and tests; Zirium Rust 1.88/stable workspace tests and Clippy, doctests/docs; fresh editable CPython 3.11–3.14 format, lint, type and full pytest checks. Each Python run has 165 passes and two existing macOS /dev/full skips.
- Focused public controls preserve native text/typed projection, empty-side sets, strict traversal, fixed-point cycles and nested limits, live handles versus scalar snapshots, and later-error buffering. Normal-thread 25,000-term parse/drop and later-parse-error cleanup pass, including the original standalone failure reproducer.
- The original four-variant Runtime and DiffQueryResult conversion contract remains unchanged. The seven-line Program ownership method has an independent scoped PASS; host disposal adds no executor, unsafe code or logical-depth cap. Core and consumer source commits are separately reviewable and pushed.
- git diff --check and zdev check base pass. Registry publication and distribution packaging remain deferred under pinned-Git development; Linux validation is not claimed.

Subsequent development CI correction: consumer commit d62830c17d603f3c77e89c0090f95569138254d5 moves registry package verification to the release boundary. Cargo archive verification resolves published Zirql alpha.5, which lacks the ownership method used by the Git integration. The independently reviewed workflow and guide changes preserve release quality gates; actionlint and local site checks/build pass. [Development CI run 38395223629](https://github.com/zayenz/zirium/actions/runs/38395223629) passes all 12 jobs, including Linux Rust checks and supported Linux/macOS Python builds. No dependency publication was needed.
