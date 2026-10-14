+++
schema_version = 1
id = "base-040"
key = "explicit-loop-state"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-039"]
+++
# Execute expression loops with simultaneous immutable state transitions

## Outcome

Shared loop expressions return a value through break and advance complete state through continue without growing active call depth.

## Context

Read the selected loop contract. Build on patterns-and-value-match blocks and base-033 frames/continuations. Existing comprehensions and recursive functions keep their semantics; this is the state-carrying loop used by the worked example.

## Boundaries

- No mutable locals, implicit unit/fallthrough, loop labels, named continue arguments, effects or duplicate interpreter.

## Done when

- [ ] Loop initializers run in outer scope; state names become fresh lexical slots. Continue evaluates every operand against old state, validates arity/contracts and installs the entire next state only on success.
- [ ] Break returns its selected expression; transfers target the innermost loop, never cross function boundaries, and invalid placement/body fallthrough fail preparation.
- [ ] Iterations spend work without accumulating frames, old states or call history. Non-tail calls/expression nesting/host reentry retain their limits; aliases and budget state survive normal exit and errors.
- [ ] Callable library examples demonstrate an accumulator beyond the active-depth ceiling, a simultaneous two-slot update, nested-loop targeting and ordinary work-limit refusal.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Add focused controls for simultaneous updates, lexical scope, wrong state/arity, nested transfer boundaries and failure cleanup; measure released state temporarily.
