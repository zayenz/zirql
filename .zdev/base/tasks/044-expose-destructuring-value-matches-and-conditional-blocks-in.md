+++
schema_version = 1
id = "base-044"
key = "zincite-patterns"
area = "base"
status = "done"
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

- [x] Textual and typed-library patterns implement nested tuples/records, renaming, ignored fields and optional/literal patterns in let, parameters and value match.
- [x] match(subject) selects ordered guarded arms and remains distinct from native match capture enumeration; contextual names and existing simple queries remain usable.
- [x] Conditional/match blocks retain lexical scope, pure native queries, runtime errors and compatible tail calls through selected final expressions.
- [x] A complete destructured report and optional guarded-match example agrees across entry points, with source locations for partial-pattern refusal, invalid guards and forbidden effects. Update user guides.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Run Zirql and the affected host’s required checks. Reuse compact examples for inline/file/typed-library agreement, located failures and later-error output buffering; record separate consumer commits and Git pins.

## Result

Zincite textual and typed programs share destructuring, guarded value matches and conditional blocks; bounded typed admission and iterative literal cleanup preserve native inputs and MiniZinc expressions.

Validation:

- Independent whole review and one focused correction recheck passed; all three findings resolved, with the exact staged consumer candidate verified.
- Fresh core (110) and Zincite (470) tests and doctests, formatting, strict workspace all-target Clippy, diff checks and zdev checks passed; shared cleanup also passed Rust 1.88 tests and strict Clippy.
- Depth-20,000 and width-100,000 typed probes refuse safely; 12 nested CLI probes, native-input and terminal-edit compatibility controls passed on the exact pinned Git runtime.
- Final site check/build and actual desktop/mobile light/dark review passed; search, keyboard focus and 938 local links plus 20 repository targets passed with no console errors.
