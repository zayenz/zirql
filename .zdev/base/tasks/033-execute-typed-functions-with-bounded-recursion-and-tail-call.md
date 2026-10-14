+++
schema_version = 1
id = "base-033"
key = "typed-tail-functions"
area = "base"
status = "done"
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

- [x] Located function declarations and calls register signatures before bodies, resolve lexical locals/names/arity/purity, and enforce runtime argument/result contracts while retaining native values.
- [x] Direct and mutual calls work. Non-tail calls consume active depth; self and mutual tail calls with the same result contract reuse a frame while each call spends work. Arguments are evaluated and checked before replacing caller locals.
- [x] Tail positions cover final expressions and selected conditional/requirement/default branches when applicable. Pending computation or incompatible return obligations retain the needed continuation.
- [x] A tail-recursive accumulator completes substantially more calls than the active-depth ceiling when work permits; equivalent non-tail recursion hits the declared depth limit. Infinite tail recursion and branching recursion end in located work/limit errors.
- [x] Native calls during recursion share the budget and bounded host reentry guard. Normal return and failure release locals without growing retained call history, deferred return checks or unsafe recursive cleanup.
- [x] Public library controls demonstrate pure native arguments/results and reject attempts to emit/edit; documented function API supports later host frontends.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Add compact public controls for accumulator tail recursion, mutual tail calls, non-tail depth, runtime type refusal, native reentry and failure cleanup. Measure retained frames/values temporarily; no timing assertions.

## Result

Execute typed pure functions with shared explicit continuations, native argument/result contracts, bounded non-tail depth and compatible self/mutual tail-frame reuse.

Validation:

- Stable format, warning-fatal all-target Clippy, workspace tests and Rust 1.88 tests pass; each test suite includes 57 public tests and six doctests.
- Focused recursion, preflight, native ownership/reentry, failure cleanup and both original correction reproductions pass; final temporary measurements demonstrate bounded live tail state and immutable payload sharing while documenting small-query overhead.
- Actual-origin site check/build and desktop/mobile guide inspection pass; corrected docs and all 55 outputs match inspected hashes.
- Independent whole review and one combined correction followed by fresh focused PASS; work-context Wa78f3b1fad6dd9e8 and both consumer Git baselines remain unchanged.
- git diff --check and zdev check base pass; original untracked files, manifests, lockfile and consumer pins are preserved.
