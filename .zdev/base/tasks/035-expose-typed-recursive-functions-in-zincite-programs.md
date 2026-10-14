+++
schema_version = 1
id = "base-035"
key = "zincite-functions"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-033", "base-031"]
+++
# Expose typed recursive functions in Zincite programs

## Outcome

Zincite users can define typed query functions over native nodes and values, with MiniZinc-shaped expressions and shared tail-call execution.

## Context

Read the common fn/signature contract, Zincite AGENTS.md and its program/CST expression lowering and semantic adapters. Implement query-defined functions, not calls that execute model functions. Translate the brief’s count_calls example to actual syntax-node primitives.

## Boundaries

- Zincite query integration only, plus necessary shared seams; no model execution, solver behavior or broadened data-edit semantics.

## Done when

- [ ] Common fn declarations and selected type spellings compose with Zincite’s expression syntax and native-query forms, lowering direct/mutual and tail calls into the shared runtime.
- [ ] Host Node/Declaration/nominal contracts retain source coordinates, enum identity and incompleteness; uncertain semantic facts do not become absence or false.
- [ ] The syntax-tree example counts two written sum calls and excludes a max call; a tail accumulator demonstrates constant active depth and ordinary budget refusal.
- [ ] Inline/file/library entry points agree, existing simple queries and later-error buffering remain compatible, and complete examples explain query functions versus model functions.
- [ ] Record separate consumer/core commits and update the pinned Git dependency.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse compact syntax-tree and tail-accumulator controls for frontend acceptance, native identity/type refusals and limit cleanup.
