+++
schema_version = 1
id = "base-039"
key = "patterns-and-value-match"
area = "base"
status = "open"
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

- [ ] Located nested name/wildcard/tuple/record/literal/presence patterns implement the selected shape, renaming and explicit ignored-rest rules in let and typed parameter positions.
- [ ] Binding values/arguments evaluate once; whole parameter admission and pattern matching complete before names become visible. Mismatches and duplicate/shadowing names give located errors without partial bindings.
- [ ] Value match evaluates its subject once, tries arms/guards in order, requires a final unguarded wildcard, and evaluates only the selected result. False guards continue; unknown/non-Boolean/host/resource errors fail.
- [ ] Lexical expression blocks compose existing conditionals, local bindings and selected match arms, retaining native handles through explicit record views and preserving compatible tail-call frame reuse.
- [ ] Existing native capture matching and public simple binding/conditional paths remain compatible; parsing/preflight/error cleanup and pattern/guard work share the budget.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Use a destructured native/constructed report and optional guarded match to prove scope, selected-arm laziness, parameter mismatch, guard failure and a tail call through a selected arm.
