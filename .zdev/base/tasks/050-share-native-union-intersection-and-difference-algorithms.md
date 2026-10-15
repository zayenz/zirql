+++
schema_version = 1
id = "base-050"
key = "shared-native-set-algebra"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Share native union, intersection and difference algorithms

## Outcome

Zirql owns native set membership and combination while hosts supply identities, compatible kinds and their established result order.

## Context

Zirql src/program.rs::ExpressionKind::Set sequences operands but ProgramHost::combine delegates the operation itself. Zirium query.rs::evaluate_set_operator and query/diff_program.rs::combine implement union/intersect/except separately. Read query-language.md Combining queries. Ordinary operations use document order, string sets use first-seen/left order, and diff selections retain canonical order and same-side validation even when empty.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [ ] Reusable shared union/intersection/difference operations have a callable typed path and common operator reading. Both operands see the same incoming selection, execute left to right and preserve permitted emitter sequencing and existing equal-precedence left associativity.
- [ ] Zirium ordinary, typed and diff combiners delegate membership and combination to Zirql. Ordinary operation sets remain in document order; string union preserves first appearance from left then right and intersection/difference preserve deduplicated left order; diff change/operation sets retain canonical order.
- [ ] Hosts retain foreign-handle, stream-kind and diff-side admission, including empty operands. Shared execution keeps located failures and charges membership, result construction and copies to the active budget; flat chains retain their existing nesting behavior.

## Validation

- Reuse ordinary/string/diff set controls for duplicates, order, empty and mismatched kinds/sides, grouped pipes and same-input operands. A small shared-library native control must exercise all three operators without host-owned set algorithms.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.
