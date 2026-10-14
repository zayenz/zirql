+++
schema_version = 1
id = "base-040"
key = "explicit-loop-state"
area = "base"
status = "done"
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

- [x] Loop initializers run in outer scope; state names become fresh lexical slots. Continue evaluates every operand against old state, validates arity/contracts and installs the entire next state only on success.
- [x] Break returns its selected expression; transfers target the innermost loop, never cross function boundaries, and invalid placement/body fallthrough fail preparation.
- [x] Iterations spend work without accumulating frames, old states or call history. Non-tail calls/expression nesting/host reentry retain their limits; aliases and budget state survive normal exit and errors.
- [x] Callable library examples demonstrate an accumulator beyond the active-depth ceiling, a simultaneous two-slot update, nested-loop targeting and ordinary work-limit refusal.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Add focused controls for simultaneous updates, lexical scope, wrong state/arity, nested transfer boundaries and failure cleanup; measure released state temporarily.

## Result

Added callable expression loops with simultaneous immutable state transitions, fixed runtime contracts and lexical break/continue. Reconciled with named fold/scan while retaining native identity, tail return obligations and bounded active execution.

Validation:

- First independent whole-task review passed without findings; snapshot Wd881c3a525334070 matched exactly before completion. Public controls cover 10,000-step accumulation, swaps, nested transfers, contract/error/budget refusal, native aliases and loop-break tail calls in named accumulators.
- Stable formatting/check/warning-fatal Clippy, stable and Rust 1.88 workspace tests with 86 integration tests and eight doctests each, diff and zdev checks passed. Native callback/depth controls use the unchanged ordinary stack and guards.
- Site checks/build passed; 88 source/build hashes match the actual twelve-view desktop/mobile light/dark guide review. All 27 search assets are nonempty; 158 local links/anchors, loop search and keyboard focus checks passed.
- A separate final reconciled sixteen-case observation used the unchanged driver through 100,000 steps. Peak extra requested bytes stayed constant for each normal/error/refusal case, and all exits returned to retained-fixture baseline. Requested System allocator bytes include observation overhead; no RSS, copy-count, native-allocation or speedup claim. Earlier original and metadata-corrected observations remain separate.
