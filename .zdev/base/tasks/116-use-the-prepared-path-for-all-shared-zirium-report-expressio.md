+++
schema_version = 1
id = "base-116"
key = "zirium-prepared-reports"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-113"]
+++
# Use the prepared path for all shared Zirium report expressions

## Outcome

Zirium shared reports execute and discover types through one prepared path, including programs without function declarations.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Inspect Zirium query/report.rs evaluate paths around 505, requires_session/value_requires_session around 2472, types_with_host/projection_types_with_host around 2793, functions.rs Session and query/program.rs. Zirql PreparedFunctions already supports an empty function set. Existing small-query measurements are described in Zirium docs/architecture/query-profiling.md.

## Done when

- [ ] Ordinary report expressions and projections, including an empty function list, use existing prepared evaluation/type discovery; the alternate route and session-selection classifier are deleted where superseded.
- [ ] Native/constructed result distinctions, relative input, lexical bindings, preflight refusals, request budgets and later-error buffering remain correct for library and CLI entry points.
- [ ] An existing small-query workload is exercised before/after to identify conspicuous overhead, with measured observations and any necessary bounded correction recorded; no new benchmark infrastructure.
- [ ] Zirium architecture and shared embedding docs explain the single path and actual boundary.

## Validation

- Reuse plain report, projection, scoped/native value, empty input and later-failure controls; run existing relevant query-profiling command on the same input.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
