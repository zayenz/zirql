+++
schema_version = 1
id = "base-033"
key = "typed-tail-functions"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-032"]
+++
# Execute typed functions with bounded recursion and tail-call frame reuse

## Outcome

The shared library executes typed pure functions, including direct/mutual recursion and constant-depth compatible tail calls.

## Context

Read the selected function and tail-call contracts. Build on prepared slots, runtime type admission and native expression hooks. Use explicit frames/continuations rather than Rust-stack recursion. This core slice needs callable library controls; consumer textual entry points are subsequent tasks.

## Boundaries

- No function values, closures, overloads, imports, termination proof or change to existing expression/host limit defaults.

## Done when

- [ ] Located function declarations and calls register signatures before bodies, resolve lexical locals/names/arity/purity, and enforce runtime argument/result contracts while retaining native values.
- [ ] Direct and mutual calls work. Non-tail calls consume active depth; self and mutual tail calls with the same result contract reuse a frame while each call spends work. Arguments are evaluated and checked before replacing caller locals.
- [ ] Tail positions cover final expressions and selected conditional/requirement/default branches when applicable. Pending computation or incompatible return obligations retain the needed continuation.
- [ ] A tail-recursive accumulator completes substantially more calls than the active-depth ceiling when work permits; equivalent non-tail recursion hits the declared depth limit. Infinite tail recursion and branching recursion end in located work/limit errors.
- [ ] Native calls during recursion share the budget and bounded host reentry guard. Normal return and failure release locals without growing retained call history, deferred return checks or unsafe recursive cleanup.
- [ ] Public library controls demonstrate pure native arguments/results and reject attempts to emit/edit; documented function API supports later host frontends.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Add compact public controls for accumulator tail recursion, mutual tail calls, non-tail depth, runtime type refusal, native reentry and failure cleanup. Measure retained frames/values temporarily; no timing assertions.
