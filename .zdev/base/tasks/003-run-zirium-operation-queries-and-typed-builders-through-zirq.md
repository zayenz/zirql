+++
schema_version = 1
id = "base-003"
key = "zirium-read-only-adapter"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-001"]
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

- [ ] Ordinary read-only expression evaluation, common collection stages and predicate composition use Zirql from both textual and builder entry points.
- [ ] Public Rust results and Python handle wrappers retain the existing types and lifetimes; saved scalar values remain snapshots.
- [ ] Existing pipeline, projection and native structured-map controls pass, with no new core dependency on Zirium.
- [ ] The ordinary expression adapter preserves current limit accounting and leaves unsupported migration portions on an explicit host-owned path.
- [ ] Attributable consumer and framework commits are recorded separately.

## Validation

- Reuse ordinary query/query_dsl tests and relevant Python query tests, plus the host's required format, lint and Rust checks.
- Run Zirql checks and inspect the dependency graph for a core-to-host dependency; no new broad compatibility matrix.
