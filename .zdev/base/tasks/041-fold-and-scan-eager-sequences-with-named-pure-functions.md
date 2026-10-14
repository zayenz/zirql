+++
schema_version = 1
id = "base-041"
key = "named-fold-scan"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-033"]
+++
# Fold and scan eager sequences with named pure functions

## Outcome

The shared library invokes named pure functions for fold and scan through the existing typed function executor.

## Context

Read the selected fold/scan contract. Extend base-033 narrowly beyond its initial no-function-values scope: a step argument is a resolved named function, not a closure or arbitrary stored value. Reuse runtime admission, frame budgets and immutable value ownership.

## Boundaries

- Eager positional input only; no anonymous functions, closures, stored/returned function values, generic user functions, lazy reducers or new sorting-selector overloads.

## Done when

- [ ] Resolve step signatures, arity and purity before iteration including empty input; evaluate items then initial once, and call step(state,item) left-to-right with runtime argument/result contracts.
- [ ] Fold returns initial for empty input and final state otherwise. Scan returns every post-step state, excludes initial and is empty on empty input. Native sequence elements retain identity.
- [ ] Earlier retained scan states and aliased inputs remain unchanged; scalar/native failures, resource exhaustion and call diagnostics preserve their meaning and locations.
- [ ] Public library controls execute named accumulator functions without new static inference, re-preparing bodies per item or deep-copying unchanged input collections.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Use one accumulator for fold/scan ordering and empty results, one retained structured state, and focused wrong-signature/non-pure/step-error refusals.
