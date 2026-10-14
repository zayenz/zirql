+++
schema_version = 1
id = "base-031"
key = "optional-values"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-030"]
+++
# Use optional lookup and lazy defaults in both query languages

## Outcome

Both consumers can report absence, inspect membership and supply defaults through one optional-value evaluator.

## Context

Read the selected bundle’s opt/default/lookup contract and the comprehension contract. Start with src/value.rs and src/reader.rs and both consumers’ structured expression lowering. Current strict accesses fail on missing data and current constructed values have no absence variant. Extend the declared runtime contract from runtime-type-contracts.

## Boundaries

- Integrates Zirql, Zirium and Zincite read-only reporting; does not expand MiniZinc data-edit serialization or solver semantics.

## Done when

- [ ] Shared optional values, occurs/absent, lookup/has_key and lazy default implement the selected distinctions, including missing versus present-absent cells and incompatible key/field errors.
- [ ] Zincite <> and Zirium null lower to the same semantics in ordinary reports, with shared optional runtime contracts and located compatibility errors. The later function frontend tasks add opt signature syntax; existing external CLI/library parameter declarations and transport remain int/string/bool.
- [ ] Present false/zero/empty values keep their meaning; skipped fallbacks do no runtime work, and arithmetic/host/budget failures propagate. Ordinary arithmetic and ordering require presence handling.
- [ ] Both report renderers preserve omission versus present absence. Existing strict access is compatible; unsupported optional values in data-edit targets fail before any write.
- [ ] Update user guides and record separate consumer commits and pinned Zirql revisions.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse a compact sparse/optional-field report for presence, defaults, type errors and later failure; add only focused controls for these new distinctions.
