+++
schema_version = 1
id = "base-079"
key = "finish-pending-formatter-release"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Finish the pending documented formatter layout for Zincite 0.1.0

## Outcome

The existing pending Boolean and forall layout work is independently verified and committed as part of the first release, with source structure and comments preserved.

## Context

Zincite has an existing four-file pending change: .zdev/base/brief.md records expanded forall conjunctions and mixed Boolean precedence; crates/zincite-fmt/src/lib.rs implements it; tests/formatting.rs adds five public layout/structure/idempotence controls; docs/reference.md describes the same rules. These current requirements must not disappear from the release simply because the earlier task queue is exhausted. Root inspected the complete delta. Begin by assessing the existing implementation and tests against the exact brief change; no new implementation or tests are expected if they already satisfy it. This task explicitly adopts these four existing tracked deltas, preserving their content unless a concrete defect requires correction. Other site and exploratory edits remain unrelated.

## Boundaries

- Keep the agreed formatting semantics, source spelling, parentheses, comments, directives and CLI write behavior. No formatter redesign, expanded operator/configuration matrix or new benchmark campaign.
- This is completion of the existing input delta, not permission to overwrite unrelated local work or message its completed chat. Start read-only in the admitted canonical checkout. If a correction is necessary, Root allocates an isolated source before any edit and retains the original four-file input.

## Done when

- [ ] The existing forall-body and mixed-precedence examples render as documented, preserve parsed expression structure/comments/operator spelling and are idempotent.
- [ ] The existing pending tests and affected required checks pass; a small complete MiniZinc example is accepted with instance-only checking when applicable, without a solver run.
- [ ] Root independently verifies the exact four-file result, records the consumer commit separately and preserves all other tracked/untracked changes.

## Validation

- Run existing focused formatter controls, required current-stable workspace format/fatal Clippy/tests, and a few actual formatter/CLI examples. Reuse current measured formatter behavior; this task adds no performance claim.
- Run diff and zdev checks for changed records; verify directly affected reference prose/site rendering without new product tests. No broad corpus or generic matrix.
