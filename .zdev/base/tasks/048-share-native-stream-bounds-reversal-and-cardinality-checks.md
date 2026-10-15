+++
schema_version = 1
id = "base-048"
key = "shared-stream-shaping-checks"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Share native stream bounds, reversal and cardinality checks

## Outcome

Native queries reuse Zirql for tail, reverse and pass-through cardinality checks, including existing Zirium text, typed and diff entry points.

## Context

The 2026-10-15 ownership audit found general stages still executed in Zirium: query.rs reverse/tail/check dispatch and truncate_stream/countable_len, plus query/diff_program.rs Reverse/Tail/Check. Zirql src/lib.rs::Stage and src/eval.rs provide Head but no corresponding shared stages. Read Zirium docs/query-language.md sections Ordering, bounds, and extrema and Checking query results. Base-047 covers constructed sort/unique/tally composition, not these native operations.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [ ] Shared callable operations and common native-reader forms implement tail(n), reverse, check, check(n), check(message) and check(n, message). Retained tail items keep their input order; zero and oversized bounds behave like the existing host operations.
- [ ] Existing Zirium paths delegate the general operation to Zirql. Successful checks return the original value; nonempty/exact-count failures retain actual and expected counts and any supplied message. Hosts retain which native values are countable, including existing map/array and diff distinctions.
- [ ] Empty-input kind checks, native handles, ranked-map order, nested work/result/item limits and located errors survive shared execution. Existing head behavior and later-error CLI buffering remain compatible.

## Validation

- Reuse native query controls for tail(0), oversized tail, reversal with duplicates, successful check(0), failing exact/nonempty checks, optional messages and a later failure after an emitter. Demonstrate the shared callable/text path with a small non-MLIR native input.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.
