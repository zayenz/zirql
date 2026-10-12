+++
schema_version = 1
id = "base-003"
key = "zirium-read-only-adapter"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-001", "base-012"]
+++
# Run Zirium operation queries and typed builders through Zirql

## Outcome

Zirium ordinary read-only queries use the shared core from textual syntax and typed Rust/Python builders while retaining native operation, type and attribute results.

## Context

Read Zirium docs/query-language.md, docs/query-dsl.md, query.rs and query/{model,parser,builder,literal,render}.rs with ordinary query and query_dsl tests. Text and builders currently share Zirium's expression model. Integrate their expression evaluation first, leaving existing statement coordination in place until the programs task. Establish both repository baselines before editing and use Zirium's required Python validation when bindings change.

## Boundaries

- Preserve source-order/set-order differences, duplicates, scalar errors, typed builder result classes and document/registry ownership.
- MLIR traversal, dialect semantics, projections and rendering stay local. Diff streams, effectful stages and fixed-point integration belong to the advanced-host task.
- Do not coerce native types, attributes or selections through JSON or replace Python builder expressions with generated query text.

## Done when

- [x] Ordinary read-only expression evaluation, common collection stages and predicate composition use Zirql from both textual and builder entry points.
- [x] Public Rust results and Python handle wrappers retain the existing types and lifetimes; saved scalar values remain snapshots.
- [x] Existing pipeline, projection and native structured-map controls pass, with no new core dependency on Zirium.
- [x] The ordinary expression adapter preserves current limit accounting and leaves unsupported migration portions on an explicit host-owned path.
- [x] Attributable consumer and framework commits are recorded separately.

## Validation

- Reuse ordinary query/query_dsl tests and relevant Python query tests, plus the host's required format, lint and Rust checks.
- Run Zirql checks and inspect the dependency graph for a core-to-host dependency; no new broad compatibility matrix.

## Result

Ordinary Zirium text and typed Rust/Python queries now use Zirql native stream stages and Boolean predicates with shared work limits, preserving native handles/results and explicit host navigation, projection, reports and effect paths. Consumer commit: 5e6dfcb697407eee860f200ba53d5fc643af79a0. Separate verified core prerequisite: 7ee35cad2f8d92cbccf29977ff4496987cf7e19d, published as resolvable non-yanked 0.1.0-alpha.1.

Validation:

- Independent whole-task verifier PASS at W24aaf9cc913a469c; no findings. Primary post-review/completion comparisons equal; full external nine-path source, index and HEAD evidence unchanged before staging. Consumer committed source bytes equal the verified candidate.
- All 44 independent commands passed: Zirql fmt/clippy/tests/diff/zdev; consumer Rust 1.88 and stable workspace/all-targets/all-features tests and Clippy, doctests and warning-free docs; actual candidate Cargo packaging.
- All rebuilt CPython 3.11-3.14 format/lint/type/full-test checks passed, each 165 passed and two existing macOS /dev/full skips. Native macOS arm64 wheel and sdist tags/licenses/twine, clean installs and native query imports passed; Linux CI and optional free-threaded checks are unclaimed.
- Focused public controls preserve exact stage and nested work allowances, root short-circuit accounting, and accepted/refused frontend nesting. Maximum-depth control uses an 8 MiB stack; the unchanged baseline also exceeds the smaller default libtest stack.
- Canonical cargo +stable package -p zirium --locked passed on clean consumer 5e6dfcb; archive VCS/source matches that commit and normalized registry dependency is exactly =0.1.0-alpha.1. No path/patch override or core-to-consumer dependency.
