+++
schema_version = 1
id = "base-044"
key = "zincite-patterns"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-039", "base-035"]
+++
# Expose destructuring, value matches and conditional blocks in Zincite

## Outcome

Zincite users can destructure immutable bindings and typed parameters and choose pure results with shared conditional blocks and guarded value matches.

## Context

Read Zincite repository guidance and query program/expression lowering, plus the selected patterns/worklists contract. Extend base-035 function syntax; reuse any general conditional support it already supplies. Preserve MiniZinc expression/CST lowering and existing if/then/else/endif; new explicit control blocks lower to the shared representation with original locations.

## Boundaries

- Zincite integration and necessary shared adapter seams only; no change to its edit/emit transaction contract, no new independent evaluator or package publication.

## Done when

- [ ] Textual and typed-library patterns implement nested tuples/records, renaming, ignored fields and optional/literal patterns in let, parameters and value match.
- [ ] match(subject) selects ordered guarded arms and remains distinct from native match capture enumeration; contextual names and existing simple queries remain usable.
- [ ] Conditional/match blocks retain lexical scope, pure native queries, runtime errors and compatible tail calls through selected final expressions.
- [ ] A complete destructured report and optional guarded-match example agrees across entry points, with source locations for partial-pattern refusal, invalid guards and forbidden effects. Update user guides.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Run Zirql and the affected host’s required checks. Reuse compact examples for inline/file/typed-library agreement, located failures and later-error output buffering; record separate consumer commits and Git pins.
