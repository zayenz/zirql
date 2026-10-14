+++
schema_version = 1
id = "base-034"
key = "zirium-functions"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-033", "base-031"]
+++
# Expose typed recursive functions in Zirium programs

## Outcome

Zirium users can define and call typed native query functions with guaranteed compatible tail calls through inline, file and library entry points.

## Context

Read the common fn/signature contract, Zirium AGENTS.md and current query program/parser/builder/projection code. Adapt the brief’s subtree_cost example to real native operations and type names; direct children mean containment, not SSA users. Use the shared runtime exclusively.

## Boundaries

- Zirium integration only, plus necessary shared adapter seams; no new edit/diff transaction contract or package publication.

## Done when

- [ ] Common fn declarations, types, lexical locals and calls lower from Zirium’s existing expression/native-query forms to the shared representation, including direct/mutual and tail recursion.
- [ ] Host-declared Operation/SSA and relevant result contracts preserve native identity and document ownership, with located wrong-argument/result errors.
- [ ] The subtree-cost example produces 8 for an unannotated root and cost-3/cost-4 leaves; malformed cost data fails. A tail accumulator crosses the active-depth ceiling under sufficient work.
- [ ] Short existing queries need no function preamble, existing preflight/purity/emission behavior remains compatible, and guides include complete executable examples.
- [ ] Record separate consumer/core commits and update the pinned Git dependency.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse a native tree example and scalar tail example for frontend acceptance, runtime type refusal and budget exhaustion; avoid a full syntax matrix.
