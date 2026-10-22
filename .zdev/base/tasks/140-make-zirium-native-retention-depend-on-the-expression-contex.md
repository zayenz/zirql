+++
schema_version = 1
id = "base-140"
key = "native-retention"
area = "base"
status = "open"
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

- [ ] Equivalent reports retain the same admitted native values with or without an unused function prelude.
- [ ] Common reading and preparation choose retention from the actual expression context. The unrelated function-mode gate is removed.
- [ ] Native owners, explicit scalar-only contexts, effect restrictions and rendering boundaries remain enforced.
- [ ] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Add one public regression for the same native report with and without an unused function; reuse native owner/effect and scalar-context controls.
- Exercise the existing small-query preparation benchmark without a new threshold.
