+++
schema_version = 1
id = "base-088"
key = "zincite-linux-cli-stdin-helper"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "high"
blocked_by = []
+++
# Handle early stdin closure in Zincite CLI test helpers

## Outcome

The required Zincite hosted workspace tests tolerate commands that deliberately exit without reading supplied stdin, while still checking their actual exit statuses and output.

## Context

Hosted CI at accepted Zincite commit f827689dc7dd9143422bddf9eef7aa2024277b31 fails in four lint CLI tests because crates/zincite-lint/tests/cli.rs run_bytes unconditionally unwraps write_all(source). Catalogue/configuration-preflight commands can close stdin before that write; Linux reports BrokenPipe. Root inspected the helper and failure evidence at https://github.com/zayenz/zincite/actions/runs/38855041901 and /private/tmp/zirql082-ci-failure.log. Current-stable fmt, fatal Clippy and prior local full tests pass; the hosted run reaches the test helper. Read the failing tests and nearby helper conventions before choosing the smallest correction.

## Boundaries

- Keep production command behavior and test status/stdout/stderr assertions. Do not skip tests, ignore arbitrary I/O failures, weaken CI or force serial execution to hide the race. No new product tests or test framework are needed.
- Work in an isolated Zincite source with separately attributable consumer and core-record commits. Preserve unrelated source and the concurrent predicate work. No registry publication, release tags or site deployment.

## Done when

- [ ] The helper tolerates only the expected early-close BrokenPipe case and still waits for and validates child results; other stdin write failures remain visible. Inspect related required CLI helpers for the same exhibited pattern without a broad test rewrite.
- [ ] The affected existing CLI controls and required workspace fmt/fatal Clippy/tests pass. The exact accepted source is pushed and the existing Linux CI succeeds, or any different concrete remaining failure is retained for correction before release acceptance.
- [ ] Root independently verifies the focused change, records separate commits and makes final release acceptance depend on the hosted correction.

## Validation

- Replay the four exhibited lint CLI controls and run existing workspace fmt, fatal all-target Clippy and tests. Add no mirrored test for the helper implementation.
- Use the existing hosted stable workflow at the exact corrected commit as Linux evidence. Keep the failed run and successful replacement identifiable; no expanding compiler/platform matrix.
- Run git diff --check and authoritative zdev checks; inspect full attributed source/record deltas.
