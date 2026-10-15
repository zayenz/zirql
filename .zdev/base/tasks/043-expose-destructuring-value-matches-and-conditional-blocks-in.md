+++
schema_version = 1
id = "base-043"
key = "zirium-patterns"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-039", "base-034"]
+++
# Expose destructuring, value matches and conditional blocks in Zirium

## Outcome

Zirium users can destructure immutable bindings and typed parameters and choose pure results with shared conditional blocks and guarded value matches.

## Context

Read Zirium repository guidance and query program/expression lowering, plus the selected patterns/worklists contract. Extend base-034 function syntax; reuse any general conditional support it already supplies. Preserve JSON-shaped record expressions and ordinary contextual binding names.

## Boundaries

- Zirium integration and necessary shared adapter seams only; no change to its edit/emit transaction contract, no new independent evaluator or package publication.

## Done when

- [x] Textual and typed-library patterns implement nested tuples/records, renaming, ignored fields and optional/literal patterns in let, parameters and value match.
- [x] match(subject) selects ordered guarded arms and remains distinct from native match capture enumeration; contextual names and existing simple queries remain usable.
- [x] Conditional/match blocks retain lexical scope, pure native queries, runtime errors and compatible tail calls through selected final expressions.
- [x] A complete destructured report and optional guarded-match example agrees across entry points, with source locations for partial-pattern refusal, invalid guards and forbidden effects. Update user guides.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Run Zirql and the affected host’s required checks. Reuse compact examples for inline/file/typed-library agreement, located failures and later-error output buffering; record separate consumer commits and Git pins.

## Result

Expose shared destructuring, guarded value matches and conditional blocks through Zirium text, Rust builders and Python reports, preserving native results and host execution contracts.

Validation:

- Independent whole review and focused correction recheck passed; rejected owned patterns are disposed iteratively through all three typed builders.
- Rust stable and 1.88: 634 tests passed per toolchain, 5 intentional ignores; strict Clippy, formatting, 3 doctests and warning-fatal rustdoc passed.
- Fresh private conventional CPython 3.11-3.14: 175 tests passed and 2 existing skips each; import/ABI identity, Ruff and ty passed against exact Git 4eaaafb.
- Inline/file/typed examples, located failures, buffering, tail behavior, nine cleanup controls and retained documentation UI checks passed; six canonical Python artifacts unchanged.
- git diff --check and zdev check base passed; post-review core snapshot and full consumer source/index/environment guard matched.
