+++
schema_version = 1
id = "base-104"
key = "sequence-neighbourhoods"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Inspect complete windows and consecutive runs

## Outcome

Users can query adjacent items and consecutive equal-key runs without manual index arithmetic.

## Context

Word adjacency currently uses one-based index arithmetic; group_by collects equal keys across the entire collection. Start with docs/examples.md, src/reader.rs and existing collection execution. Projected operation names provide a Zirium use. Reuse existing structural comparison rather than inventing key equality. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Initial sources are positional sequences. Defer padding, stride options, prefix/suffix splitting and arbitrary selector callbacks.
- Keep this slice in Zirql and its local examples; record concrete consumer adoption requirements without changing consumer repositories.
- No Gecode dependency, symbolic CP lowering, first-class function system, publication or deployment.

## Done when

- [ ] windows(items, size) and the pipeline suffix windows(size) return eager complete overlapping positional windows with stride one. Size must be positive; fewer items than size yields an empty sequence.
- [ ] runs_by(items, "field") and its pipeline suffix group consecutive equal field keys into records with key and items, retaining input order and repeated members. Missing fields fail under existing access rules; empty input produces no runs.
- [ ] A neighbouring-word example and repeated operation-name report distinguish overlapping windows, consecutive runs and global grouping. Focused checks cover an invalid size and budget refusal for window expansion.
- [ ] Update the relevant public language guide and callable/embedding documentation with the implemented contract, supported host/example entry point, limitations and a runnable example with expected output. Document necessary consumer adoption separately.

## Validation

- Use the focused behaviour checks described in Done when; run the documented local example and compare its result.
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- git diff --check
- zdev check base --format json
