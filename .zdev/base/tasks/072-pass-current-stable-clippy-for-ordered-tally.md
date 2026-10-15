+++
schema_version = 1
id = "base-072"
key = "ci-tally-closure-borrows"
area = "base"
status = "open"
complexity = "routine"
afk = true
priority = "high"
blocked_by = []
+++
# Pass current stable Clippy for ordered tally

## Outcome

The ordered native tally passes current stable Clippy without redundant closure borrows or changed behavior.

## Context

Verified collection commit b92aa4c passes local Rust1.98.1 checks and CI Rust1.88 tests, but CI run38801651581 stable Rust1.99 rejects src/eval.rs:496,516 .ok_or_else(&storage_error) under needless_borrows_for_generic_args. The closure captures references and can be passed by value. Reproduce with explicit Rust1.99 alongside existing toolchains, remove only the two needless borrows and retain checked storage errors and budget behavior. This is a narrow CI follow-up to the completed collection task, not a second collection review.

## Boundaries

- No warning suppression, toolchain/MSRV/default change, query behavior, dependency, new test or consumer change. Preserve unrelated homepage/site/CSV work; use isolated source and separate verified commit.

## Done when

- [ ] Both ordered native tally storage failures pass the reusable closure by value while retaining their current locations, categories and checked arithmetic.
- [ ] Current stable Rust1.99 formatting/Clippy with warnings denied and existing workspace tests pass; declared Rust1.88 tests remain passing, and the pushed follow-up CI is green.

## Validation

- Reproduce the two CI diagnostics with cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings before editing; run the same command after the minimal fix.
- Run cargo fmt --all -- --check, cargo test --workspace --locked, cargo +1.88.0 test --workspace --locked, git diff --check and zdev check base --format json; no new product tests.
- Independent focused review confirms the two-line diff preserves tally failure behavior; check the actual pushed CI run.
