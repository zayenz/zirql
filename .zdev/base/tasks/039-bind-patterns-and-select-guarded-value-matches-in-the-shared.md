+++
schema_version = 1
id = "base-039"
key = "patterns-and-value-match"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-033", "base-031"]
+++
# Bind patterns and select guarded value matches in the shared runtime

## Outcome

One shared pattern implementation supports immutable bindings, destructured function arguments and ordered value-match expressions with lexical blocks.

## Context

Read the final selected patterns/worklists contract and worked graph in the brief. Extend base-033’s prepared function/body representation and base-031 optional values. Start at src/value.rs, src/program.rs, src/reader.rs, src/runtime_types.rs and src/matching.rs; current NativeMatch enumerates captures and must retain its semantics. This task delivers callable library behavior, with text frontends in later tasks.

## Boundaries

- Extend the existing evaluator/prepared representation; no reflected Rust fields, bound rest copies, exhaustive type inference, new user variants or effectful blocks.

## Done when

- [x] Located nested name/wildcard/tuple/record/literal/presence patterns implement the selected shape, renaming and explicit ignored-rest rules in let and typed parameter positions.
- [x] Binding values/arguments evaluate once; whole parameter admission and pattern matching complete before names become visible. Mismatches and duplicate/shadowing names give located errors without partial bindings.
- [x] Value match evaluates its subject once, tries arms/guards in order, requires a final unguarded wildcard, and evaluates only the selected result. False guards continue; unknown/non-Boolean/host/resource errors fail.
- [x] Lexical expression blocks compose existing conditionals, local bindings and selected match arms, retaining native handles through explicit record views and preserving compatible tail-call frame reuse.
- [x] Existing native capture matching and public simple binding/conditional paths remain compatible; parsing/preflight/error cleanup and pattern/guard work share the budget.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Use a destructured native/constructed report and optional guarded match to prove scope, selected-arm laziness, parameter mismatch, guard failure and a tail call through a selected arm.

## Result

Delivered callable shared located patterns for immutable lexical bindings, whole typed parameters and ordered guarded value matches through the existing prepared runner. Nested tuple/record/literal/presence forms retain exact shape, renamed fields and explicit ignored-rest semantics. Whole arguments are admitted before atomic parameter binding; duplicate/shadowed names fail preflight, false guards continue and Unknown/non-Boolean/host/resource failures propagate. Selected blocks/arms retain compatible tail reuse. Explicit native record/tuple views preserve owners, coordinates and child contracts. Existing name-only declarations, simple bindings/conditionals and native capture enumeration remain compatible. Textual consumer frontends remain base-043/base-044; no consumer/pin or publication changes.

Validation:

- Independent first whole review PASS, no findings or corrections; W7418a6251e981404 fresh opaque completion comparison equal and all three external repository guards unchanged. Evidence /private/tmp/zirql039-whole-verifier/manifest.json. Original advanced plan and baseline retained through conflict-free source reconciliation with accepted base-038; all 14 source hashes unchanged.
- Independent stable fmt, warning-fatal workspace/all-target Clippy, stable and Rust 1.88 workspace suites each 73 integration tests plus six doctests, cached/uncached diff checks and zdev check passed. Focused public controls cover nested native/constructed patterns, atomic admission, optional/ordered guard failures, scope/shape refusals, budget cleanup, native source identity and deep iterative disposal; 10000-step selected-arm/block tail call succeeds at active_depth=1.
- Independent documentation check/build passes with all search assets nonempty. Actual built UI checks three changed guides in light/dark desktop/mobile (12 screenshots), 142 links/anchors, new API search result and navigation, Escape/focus, canonical paths and clean console. All 88 UI-reviewed source/generated/output hashes match the final destination; no source/index/consumer/Python validation writes.
