+++
schema_version = 1
id = "base-029"
key = "computation-baseline"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Measure repeated evaluation and value ownership costs

## Outcome

A reproducible baseline identifies preparation, execution and copying costs for the upcoming runtime work.

## Context

Read the brief’s execution/measurement proposal and base-010 results when available. Inspect src/value.rs, src/projection.rs and Bindings::with_local in src/program.rs. Existing adoption measurements compare combined reports with separate processes; this task measures repeated analysis and value handling within one request.

## Boundaries

- No runtime changes, raised limits, corpus campaign or persistent profiling service.

## Done when

- [ ] Record exact revisions, fixed representative inputs at a few sizes, build/environment, phase timings, repeated samples/spread and available peak memory for a small query, repeated projection and repeated large-binding access.
- [ ] Temporary instrumentation attributes body analysis, scope metadata and value-copy costs without treating source observations as measured bottlenecks.
- [ ] Retain reproducible commands and baseline evidence in the task result; separate setup, completed runs and limit refusals.

## Validation

- Check output/status parity and run the existing named measurement drivers or a small scoped adaptation; record unavailable measurements explicitly.
- No new product tests. Run record checks and the relevant checks if a measurement driver changes.
