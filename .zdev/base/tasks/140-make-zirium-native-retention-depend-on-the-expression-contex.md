+++
schema_version = 1
id = "base-140"
key = "native-retention"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Make Zirium native retention depend on the expression context

## Outcome

An unrelated unused function declaration does not change whether a report retains native values.

## Context

In Zirium, `report [n for n in input] | json` fails until an unused `fn id(x:int)->int{x}` is prepended. query/parser.rs function_mode derives from has_control_values and controls retains_native_values and normalize_outer; query/functions.rs then selects retained versus projecting wrappers. Mixed native reports already have explicit rendering support.

## Boundaries

- Do not make domain-native values implicitly scalar or remove real host admission boundaries.

## Done when

- [x] Equivalent reports retain the same admitted native values with or without an unused function prelude.
- [x] Common reading and preparation choose retention from the actual expression context. The unrelated function-mode gate is removed.
- [x] Native owners, explicit scalar-only contexts, effect restrictions and rendering boundaries remain enforced.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Add one public regression for the same native report with and without an unused function; reuse native owner/effect and scalar-context controls.
- Exercise the existing small-query preparation benchmark without a new threshold.

## Result

Equivalent Zirium reports retain the same admitted native values with or without an unused function declaration. Scalar contexts and native ownership checks remain enforced.

Validation:

- Required offline locked Rust formatting, Clippy and tests passed in affected repositories; Zirium doctests and warnings-denied docs passed. Focused public controls passed.
- Astra medium reviewed added prose using unslop and zayenz-writing-style. Both consumers use canonical Zirql 8fe2460c21d6b36a758d7c788a572efd475c71fa.
- Independent verifier passed; exact core snapshot and consumer states matched before completion.
