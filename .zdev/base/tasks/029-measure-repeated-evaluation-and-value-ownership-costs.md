+++
schema_version = 1
id = "base-029"
key = "computation-baseline"
area = "base"
status = "done"
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

- [x] Record exact revisions, fixed representative inputs at a few sizes, build/environment, phase timings, repeated samples/spread and available peak memory for a small query, repeated projection and repeated large-binding access.
- [x] Temporary instrumentation attributes body analysis, scope metadata and value-copy costs without treating source observations as measured bottlenecks.
- [x] Retain reproducible commands and baseline evidence in the task result; separate setup, completed runs and limit refusals.

## Validation

- Check output/status parity and run the existing named measurement drivers or a small scoped adaptation; record unavailable measurements explicitly.
- No new product tests. Run record checks and the relevant checks if a measurement driver changes.

## Result

Measured in-request expression analysis, scope metadata and binding-copy costs without runtime or consumer changes; retained exact source, raw paired samples, archive replay and explicit measurement limits.

Validation:

- Independent whole-task PASS with no findings at W4403b987e477cc32; all 484 frozen hashes, 96 raw records, statistics, parity and all three repository states independently checked.
- Core rustfmt, warning-fatal workspace Clippy, 40 integration tests and 4 doctests, diff/zdev checks and private Rust/Python checks pass.
- Named resume, configuration mismatch, malformed-record retry, failure/timeout, target-free analysis and exact archived-source rebuild pass. No repository validation writes; private bytecode cache retained outside repositories.

### Baseline evidence

Measured on 2026-10-14 at `948ded4ff5157f84894129be7fdc4a4fe8cbc338`,
source tree `a0ba5ad12ca4c4c0df9490b30b07dfa74bc463ab`. Locked release builds
use Rust 1.98.1 on an Apple M1 Max, 10 cores and 64 GB, with arm64 macOS
26.6.2. No runtime, consumer or product-test changes were introduced.

Five child batches per case/variant follow separate warmup children, in a
serial shuffled schedule with seed 29. The 96 terminal records comprise 70
measured successes, 10 ordinary work-limit refusals and 16 warmups, with no
failures or unfinished work. Clean and instrumented outputs/statuses agree.
Explicit probe limits are 1,000,000 work, 100,000 results, 100,000 stream items
and nesting 64; Zirql supplies no host defaults.

The clean evaluator phase includes public preflight. Parsing is separate, and
the private JSON phase includes conversion, serialization and dropping returned
values. Startup, fixture construction and later AST/output teardown are outside
these timers. Statistics describe five batches, not independent samples for
every request. Binding ASTs are constructed before timing, so their empty parse
timer is omitted.

| Case | Parse median, µs | Evaluation median (min..max), µs | JSON median, µs | Peak child RSS median (min..max), MiB |
| --- | ---: | ---: | ---: | ---: |
| Small constructed report | 2.155 | 1.795 (1.752..1.822) | 0.583 | 2.328 (2.312..2.344) |
| Projection, 10 rows | 2.645 | 77.839 (77.256..78.307) | 6.217 | 2.609 (2.547..2.625) |
| Projection, 100 rows | 3.965 | 637.596 (631.585..639.881) | 58.773 | 2.797 (2.781..2.844) |
| Projection, 1,000 rows | 10.791 | 6302.125 (6298.875..6363.458) | 631.167 | 4.250 (4.234..4.453) |
| Eight binding reads, 8 strings | n/a | 7.811 (7.795..7.933) | 0.546 | 2.172 (2.156..2.188) |
| Eight binding reads, 64 strings | n/a | 34.697 (34.597..34.908) | 0.556 | 2.188 (2.188..2.203) |
| Eight binding reads, 256 strings | n/a | 127.670 (127.429..130.023) | 0.647 | 2.281 (2.281..2.297) |

Projection retains 16 outer names; binding strings contain 128 payload bytes.
The 256-element request spends 267,727 work units. Its 8,192-element companion
refuses under the same limits and is excluded from successful timing statistics.

Temporary instrumentation observes 3,007 entries to the instrumented expression
analysis functions and 16,000 copied outer-map entries per 1,000-row projection.
The successful 256-element binding request copies 262,144 string-payload bytes
across eight reads. These counters do not measure total allocation. Observer
cost raises successful evaluation medians by 1.126 to 1.828 times; inclusive
observer timers overlap and cannot be summed or subtracted as clean phase shares.

Exact-child `wait4` peak RSS is recorded in macOS bytes. It includes startup,
fixtures and transient results, including an unused pure fixture for the
small/projection cases. It does not establish live heap or memory reclamation.
All phase samples, commands, environment and source/input/binary hashes remain
under `/private/tmp/zirql029-author/`; raw data and analysis are in `runs/final/`,
and `evidence-sha256.json` binds 484 files. Historical base-010 timings retain
their original revisions.

Reproduce analysis without invoking targets:

```sh
uv run --script /private/tmp/zirql029-author/bench.py analyze --name final
```

Convenience build/collection commands in `commands.json` require the recorded
source revision. After canonical source changes, rebuild the retained archive:

```sh
python3 /private/tmp/zirql029-verifier/reproduce.py
```

That checked replay remaps dependencies and transformations to `original-source`,
builds both locked release variants, checks the reconstructed observed source
byte for byte and compares a binding result. It does not recollect timings.
Use a fresh run name for new collection; never mix changed sources/binaries
with the completed baseline. Independent evidence and replay logs remain under
`/private/tmp/zirql029-verifier/`.
