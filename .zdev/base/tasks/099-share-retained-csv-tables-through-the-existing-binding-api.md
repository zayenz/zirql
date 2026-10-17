+++
schema_version = 1
id = "base-099"
key = "csv-shared-input-performance"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Share retained CSV tables through the existing binding API

## Outcome

The CSV example exposes its retained immutable tables through read_shared_binding, reducing the initial owned table copy while keeping full runtime admission and the existing CLI behavior.

## Context

The analysis kept a temporary Table owner as Arc<ConstructedValue> and implemented the existing ValueQueryHost::read_shared_binding hook. Million-row count fell from a buffered run of 2.626 s / 1.573 GiB RSS to 1.711 s / 1.047 GiB. The existing example uses owned tables and read_binding copies them. The temporary implementation and raw results are under /private/tmp/zirql-perf-20261017/shared-probe and runs/shared-binding. Start with examples/csv.rs Table, decode_table and ValueQueryHost. Read docs/csv-host.md and docs/runtime-limits.md. The large probe used explicit larger allowances; the public teaching example must retain its small defaults.

## Boundaries

- Preserve the CSV interface, exact output and error locations, byte/item/work/result/nesting defaults, named/implicit inputs and buffered failure behavior. Share only immutable tables owned by this invocation.
- Use the existing Arc/read_shared_binding API, without core API changes, dependencies or a new cache. Keep owned callback fallback usable. Charge actual owner allocation/sharing and retain required runtime admission.
- Update the implementation guide and affected assertions so the example teaches its actual ownership path. Do not publish the changes or turn the temporary large-data probe into a permanent benchmark suite.

## Done when

- [x] Named and implicit table bindings use retained immutable owners through the existing shared hook, with unchanged CLI results and default limits.
- [x] The source and CSV host guide coherently explain sharing, admission and owned fallback without assuming prior knowledge. Existing example tests remain useful; no new internal call-count tests.
- [x] A matched temporary large-input count/report check confirms output equivalence and reduced memory relative to the original host; note RSS scope and timing boundaries.
- [x] Required source checks, including the existing example tests, pass.

## Validation

- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace.
- Run git diff --check; coordinator runs zdev check base --format json.
- Use focused existing public-behavior tests and temporary matched performance measurements; no timing assertions, internal counters, new benchmark framework or broad test matrix.

## Result

CSV tables use retained Arc owners through read_shared_binding; the implementation guide explains sharing, full admission and owned current-input fallback.

Validation:

- Independent Astra/high verification passed; snapshot W05e730dc49e6c00d remained equal.
- Formatting, warning-fatal workspace Clippy, workspace tests, four existing CSV example tests, diff checks and zdev check passed.
- 48 matched large-input samples passed with identical outputs. Million-row count evaluation 1.501s to 0.637s; peak process RSS 1689010176 to 1124548608 bytes. Report RSS 2742484992 to 2118205440 bytes. Evaluation excludes loading/rendering; buffered run includes them and excludes stdout writing. Enlarged probe limits only.
