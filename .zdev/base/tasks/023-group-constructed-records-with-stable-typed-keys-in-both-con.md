+++
schema_version = 1
id = "base-023"
key = "stable-record-groups"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-019"]
+++
# Group constructed records with stable typed keys in both consumers

## Outcome

Shared group_by forms ordered groups that compose with comprehensions and reducers in Zirium and Zincite reports.

## Context

Read the grouping section of [extension research](../background/language-extensions.md) and example 1's grouping variation. Existing tally counts strings and map_by rejects repeated keys; neither preserves all rows grouped by an explicitly typed field.

## Boundaries

- Use group_by(rows, "field") over positional record arrays with one static field selector. Initially permit homogeneous string, integer or nominal-member keys; preserve nominal identity.
- Return existing constructed arrays/records with key and items fields. No callback selectors, automatic string coercion, joins or implicit conversion of keyed arrays.

## Done when

- [ ] Groups follow first occurrence and members preserve input order/duplicates; empty input returns an empty array. Missing fields or incompatible key types give located failures.
- [ ] Validation/type discovery/copying and evaluation support grouped values with budgeted comparisons and allocations; no host implements grouping independently.
- [ ] The example grouping returns key 1 with double/mix then key 0 with plain. A Zincite record-array report groups repeated machine keys and computes per-group counts/totals.
- [ ] Each consumer documents and executes a short inline grouping report and its file/library equivalent with unchanged rendering/completeness policy.

## Validation

- Use one deliberately non-sorted repeated-key control and one missing/incompatible-key refusal; a native nominal-key control checks identity without adding new host enum resolution.
- Run required checks in Zirql and each affected consumer and their focused report tests.
