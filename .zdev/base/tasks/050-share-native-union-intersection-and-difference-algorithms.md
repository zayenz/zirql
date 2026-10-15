+++
schema_version = 1
id = "base-050"
key = "shared-native-set-algebra"
area = "base"
status = "done"
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

- [x] Reusable shared union/intersection/difference operations have a callable typed path and common operator reading. Both operands see the same incoming selection, execute left to right and preserve permitted emitter sequencing and existing equal-precedence left associativity.
- [x] Zirium ordinary, typed and diff combiners delegate membership and combination to Zirql. Ordinary operation sets remain in document order; string union preserves first appearance from left then right and intersection/difference preserve deduplicated left order; diff change/operation sets retain canonical order.
- [x] Hosts retain foreign-handle, stream-kind and diff-side admission, including empty operands. Shared execution keeps located failures and charges membership, result construction and copies to the active budget; flat chains retain their existing nesting behavior.

## Validation

- Reuse ordinary/string/diff set controls for duplicates, order, empty and mismatched kinds/sides, grouped pipes and same-input operands. A small shared-library native control must exercise all three operators without host-owned set algorithms.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.

## Result

Moved native union/intersection/difference membership and combination into Zirql; Zirium text, typed and diff paths delegate while retaining identity, established order and kind/side admission.

Validation:

- Independent whole review 1/correction 0 passed the full 21-path change; native snapshot and both repository guards remained unchanged.
- Stable and Rust 1.88 formatting, strict Clippy and tests passed: core 111 integration tests plus 8 doctests; host 643 tests with 5 existing ignored tests and 3 stable doctests; warning-denied rustdoc passed.
- Private CPython 3.11 through 3.14 full suites each passed 175 tests with 2 skipped; Ruff, ty and CLI controls passed; canonical Python artifacts unchanged.
- Actual pinned Git source matches all 20 frozen core files. Production-base site checks/build and rendered desktop light/dark/mobile, API search and keyboard navigation passed; 19 pages and 879 internal links checked without errors.
- Both repositories cached/unstaged git diff checks and authoritative zdev check passed.
