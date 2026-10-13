+++
schema_version = 1
id = "base-026"
key = "parser-error-cleanup"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Release native expression trees safely on shared reader errors

## Outcome

Malformed shared expressions return located errors even after a long valid flat native set expression; parser cleanup cannot overflow the stack.

## Context

The 2026-10-13 independent code review reproduced an abort in the shared reader at core revision 6fef2436a311b6f83379d129d3000db0b494b314. A ValueReaderHost recognizing union, a program budget with work/results 100000000 and nesting 64, and input followed by 100000 repetitions of " union input" and a trailing " +" aborts during cleanup. The same prefix parses successfully without the suffix; a trailing " union" returns a located error. Read Reader::expression in src/reader.rs (the right-operand error path around line 419), native_sets, NativeGuard and dispose_native_value. General value/container parsing still owns native trees through ordinary recursive drop on some error paths. This is a parser lifetime issue, not a need to reject supported flat sequencing.

## Boundaries

- Preserve accepted flat native set sequencing, ordinary nesting limits, native payload ownership and original error locations.
- Reuse or extend focused iterative release machinery; do not introduce a generic AST ownership framework or change language semantics.
- Coordinate with the active base-021 source owner before overlapping edits. Consumer changes are limited to disposal integration if the shared public ownership contract requires them.

## Done when

- [ ] Malformed scalar tails, missing container terminators and budget failures after a retained flat native expression return a located QueryError without process abort or leaked retained trees.
- [ ] Successful native expression parsing and existing flat-sequence behavior remain compatible; the public return-value disposal contract and affected host callers are checked.
- [ ] Focused regression evidence exercises the demonstrated error cleanup, and required core plus any affected consumer checks pass.

## Validation

- Reuse tests/native_programs.rs shared-reader controls; add only a focused regression that can detect the cleanup failure. Use an isolated subprocess for an aborting control if needed rather than large checked-in fixtures.
- Replay the long-flat-prefix reproducer with successful, malformed scalar and malformed native tails; verify error returns separately from successful-tree cleanup.
- Run the brief required Rust, diff and zdev checks, and required checks for any consumer actually changed.
