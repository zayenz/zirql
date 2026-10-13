+++
schema_version = 1
id = "base-023"
key = "stable-record-groups"
area = "base"
status = "done"
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

- [x] Groups follow first occurrence and members preserve input order/duplicates; empty input returns an empty array. Missing fields or incompatible key types give located failures.
- [x] Validation/type discovery/copying and evaluation support grouped values with budgeted comparisons and allocations; no host implements grouping independently.
- [x] The example grouping returns key 1 with double/mix then key 0 with plain. A Zincite record-array report groups repeated machine keys and computes per-group counts/totals.
- [x] Each consumer documents and executes a short inline grouping report and its file/library equivalent with unchanged rendering/completeness policy.

## Validation

- Use one deliberately non-sorted repeated-key control and one missing/incompatible-key refusal; a native nominal-key control checks identity without adding new host enum resolution.
- Run required checks in Zirql and each affected consumer and their focused report tests.

## Result

Added shared stable typed record grouping and report integration in Zirium and Zincite. Framework API commit 8c859a36f2d7187f78e478ec269e4a7ec411f79d; Zincite b067bae798e2657d48b5d1de2b2ac7ac2b3cb33b; Zirium 86ca3d5c60c4aa0364983d3c67045df13a128296. All pushed; consumers use the reviewed Git dependency.

Validation:

- Independent core and whole-task reviews passed without corrections. All four completion conditions passed, including ordering, typed key refusals, budgets, inline/file/library agreement and buffered failures.
- All 48 independent required checks passed, including Rust formatting, Clippy, workspace tests, doctests and four rebuilt CPython 3.11–3.14 environments (169 passed, 2 existing skips each).
- Documentation builds and actual navigation, search, theme and mobile checks passed across Zirql, both Zincite site configurations and Zirium. Fresh rendered files matched the reviewed builds.
- Zirium exact-head CI run 38482501346 passed all 12 jobs on attempt 2; the initial Maturin archive download failed before wheel compilation.
