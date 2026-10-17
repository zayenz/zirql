+++
schema_version = 1
id = "base-097"
key = "borrowed-lookup-performance"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "high"
blocked_by = []
+++
# Select retained lookup and indexed values without whole-container copies

## Outcome

Repeated lookup, has_key and positional indexing select retained constructed values without copying and revalidating the complete collection on every access.

## Context

The 2026-10-17 analysis measured N present-key lookups in an N-entry saved dictionary: 19.385 ms at N=200 and 128.023 ms at N=400; strict dictionary indexing took 1.580 and 3.391 ms. Keyed lookup took 301.747 ms at N=400. src/value/execution.rs materializes operands before optional access; RuntimeValue::materialize copies shared containers and copy_size rechecks dictionary labels or keyed axes. Start with existing select_record_field, optional_access, RuntimeValue child selection and declaration propagation. The small CSV reader is a third consumer; preserve the shared native/constructed evaluator. Retained baseline records and profiling are under /private/tmp/zirql-perf-20261017. Read the brief execution/budget contracts and docs/runtime-limits.md.

## Boundaries

- Preserve successful results, written evaluation order, absence and strict missing-key behavior, runtime contracts, native ownership, coordinates and required admission of invalid externally supplied values. Do not bypass type/shape validation merely because a value is retained.
- No new public API, language syntax, optimizer, consumer repository changes, version updates, releases or deployment. Keep native paths compatible; optimize the supported constructed path directly.
- Keep the host allowances unchanged. Newly avoided copies and their charges may disappear; charge selected owned output and any new scratch before allocation. Explain the changed budget spending in docs/runtime-limits.md rather than preserving fictitious copy charges or silently skipping actual work.

## Done when

- [ ] Present and missing optional dictionary, positional and keyed accesses retain existing results and contracts; has_key avoids materializing an unused value; strict positional access retains its located refusals.
- [ ] Saved aliases, declared selected children and native/constructed boundary behavior remain correct through the existing public entry points. Add only focused public coverage needed for newly selected paths or a concrete uncovered defect.
- [ ] Matched temporary dictionary/keyed and positional access sweeps verify outputs and demonstrate removal of repeated whole-container copying. Retain raw samples, phase boundaries and source/binary identities outside the source tree; report refusals separately.
- [ ] Required source checks pass and the runtime-limits documentation describes actual charging.

## Validation

- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace.
- Run git diff --check; coordinator runs zdev check base --format json.
- Use focused existing public-behavior tests and temporary matched performance measurements; no timing assertions, internal counters, new benchmark framework or broad test matrix.
