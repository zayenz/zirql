+++
schema_version = 1
id = "base-030"
key = "runtime-type-contracts"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Declare and enforce host runtime type contracts

## Outcome

Hosts can describe domain types and operation contracts, and shared runtime admission checks values without erasing native identity.

## Context

Read the selected next-bundle contract. Start at ValueType/ConstructedValue in src/value.rs, ParameterType in src/program.rs and ProjectionHost in src/projection.rs. Existing facts include heterogeneous positions and deferred knowledge; they are not complete interface declarations. Use the two native ownership controls in tests/native_programs.rs.

## Boundaries

- No native-value serialization, host model type inference, recursive user data types or universal object registry.

## Done when

- [x] A small shared declaration API covers the selected scalar/container/nominal/native contracts, reusing existing host kinds and runtime shape checks; core remains independent of both consumer crates.
- [x] Public admission and checked operation invocation reject incompatible arguments/results, including nested elements and distinct nominal types, with located errors under one budget.
- [x] Borrowed-source and document-handle host controls demonstrate typed native arguments/results and preserved identity; existing heterogeneous values and APIs remain accepted outside incompatible typed interfaces.
- [x] Document which checks happen once on immutable admitted values and which data-dependent facts still need runtime checks; no whole-program static checker is introduced.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Extend existing native/value controls for one valid typed native operation and focused wrong argument/result and nominal-identity refusals.

## Result

Added opt-in runtime declarations, actual-value admission and checked native operation calls with located errors, one active budget and preserved native identity.

Validation:

- Independent whole-task review and focused correction recheck passed; duplicate declaration diagnostics are admitted before allocation while existing value API costs remain unchanged.
- Stable and Rust 1.88.0 locked workspace tests passed, including 45 integration tests and four doctests; formatting and warning-fatal workspace Clippy passed.
- Focused controls cover borrowed source and document-handle identity, native sequences, nested and nominal incompatibilities, argument/result/arity locations, native failures and continuing-budget refusal.
- Site check/build passed; independent rebuild matched all 55 outputs and 423 local links/anchors. Actual desktop/mobile review passed for both changed guides; styles/assets were unchanged.
- git diff --check and zdev check base --format json passed. Both consumers, their Git pins and 36 unrelated assets/logs were preserved.
