+++
schema_version = 1
id = "base-030"
key = "runtime-type-contracts"
area = "base"
status = "open"
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

- [ ] A small shared declaration API covers the selected scalar/container/nominal/native contracts, reusing existing host kinds and runtime shape checks; core remains independent of both consumer crates.
- [ ] Public admission and checked operation invocation reject incompatible arguments/results, including nested elements and distinct nominal types, with located errors under one budget.
- [ ] Borrowed-source and document-handle host controls demonstrate typed native arguments/results and preserved identity; existing heterogeneous values and APIs remain accepted outside incompatible typed interfaces.
- [ ] Document which checks happen once on immutable admitted values and which data-dependent facts still need runtime checks; no whole-program static checker is introduced.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Extend existing native/value controls for one valid typed native operation and focused wrong argument/result and nominal-identity refusals.
