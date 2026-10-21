+++
schema_version = 1
id = "base-130"
key = "coherent-ordering-keys"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-125"]
+++
# Use one structural ordering contract for value and native operations

## Outcome

Existing sort, sort_by and extrema forms order the same actual keys identically across constructed values, native selections and dictionaries, with stable ties and successful empty inputs.

## Context

Read the brief's paired-operation coherence decision and existing collection language contract. In the post-base-125 core, src/value.rs sort_value/ordering_key/check_ordering_types/extremum_position, src/value/identity.rs compare_constructed_values, src/ordering.rs order_native_by and src/mapping.rs sort_map_by_value still implement different admitted key domains. Arrays/selectors reject values already ordered structurally as set elements/dictionary keys; Zirium native maps admit Boolean/null and exact mixed numeric keys; native selector Count/Text keys and typed keys take distinct paths. Zincite program/native.rs and Zirium query/report.rs, query.rs and query/diff_program.rs adapt these paths. Zirium alone refuses empty native extrema. Base-125 preserves the old language except its narrow selector-free Min/Max routing regression repair; this follow-up intentionally changes the inconsistent contracts.

## Boundaries

- Reuse compare_constructed_values/native_values_compare and the existing fallible sort/extrema machinery; add only the concrete lossless unsigned leaf adaptation needed by native JSON/count keys. No user comparator callbacks, generic comparator framework, second evaluator, legacy-policy switch or compatibility wrapper.
- Keep native subjects, owners, source/diff side and explicit native default projections. Opaque domain objects need a supported host ordering or an explicit projection; printed names never replace identity.
- Keep the existing operation/result shapes and supported forms, rather than adding every ordering overload to every collection. Constructed single extrema remain optional elements; native extrema retain zero/one or all matching subjects. Keyed arrays retain explicit coordinates and require explicit conversion for ordering.
- Do not broaden Zincite's closed data-edit subset or effects. Docs change with this slice.

## Done when

- [x] Every existing default/field/value/query selector form uses the same structural key order: absence, Boolean, integer, finite float, string, coherent nominal member and supported compound values follow the established constructed kind order; arrays/tuples remain positional and sets/dictionaries canonical by membership/associations. Actual absent keys are valid; a missing field or invalid selector cardinality remains a located error.
- [x] The same keys receive the same order through native and constructed representations. Native unsigned integers retain their full u64 range and compare losslessly in the integer domain, including against signed integers; they are never narrowed, stringified or rounded through f64. Numeric kinds otherwise keep the shared structural distinction.
- [x] Sort remains stable in either direction; single extrema choose the first tied original subject and all-ties forms retain input order/duplicates. Every selected key is evaluated/admitted before sorting or choosing extrema, so later failures and contradictory nominal metadata still fail.
- [x] Empty sort and extrema succeed after normal kind/effect/selector preflight. Constructed single extrema return typed absence and native extrema return an empty selection retaining its admitted kind/element schema; Zirium's historical nonempty refusal is removed.
- [x] The common reader, callable/typed builders and existing function-return/native routes expose equivalent accepted keys and results for their supported forms, without frontend-specific admission flags.
- [x] Update the combined ordering reference, Rust embedding/API documentation and affected Zincite/Zirium guides in the same task; name intrinsic default-projection/result-shape distinctions and remove obsolete host empty/key policies.

## Validation

- Reuse existing ordering, dictionary and empty extrema controls. Add a few public regressions that distinguish the old paths: a shared compound/mixed/absent key ordering, a native exact unsigned boundary, stable first/all ties and an empty native extrema result with a declared schema. Do not multiply these across every host/container/syntax.
- Run the brief's required Rust checks in each changed repository and the affected host/Python checks. Run affected documented examples and docs content/link checks; git diff --check and zdev check base --format json for record changes. Retain separate framework/consumer commits and reviewed Zirql dependency pins.

## Result

Unified structural sort/extrema keys, exact native unsigned integers, all-key admission, stable ties and successful empty native extrema. Removed scalar-category duplication; retained native projections/owners/side and result shapes. Canonical Zirql pin05c25a3d5f34ad431f573c55ddb0b7105a58b395; separately reviewed Zincite8d661febf7ae1460c7d98e0539e606fbe2309b5c and Zirium0cdfa83edfa8c923fd139f9a2b6de111e4987bd9. Docs updated together.

Validation:

- Full independent review plus one vetted documentation correction and fresh focused PASS at W143b5c5a0237df94; exact core and consumer Git states retained.
- Required Rust fmt/Clippy/tests across all3, Zirium doctests/warnings-docs, rebuilt Python Ruff/ty180passed2skipped, documented examples and all3site check/build/links passed. Correction-only docs/pin recheck and offline locked metadata passed.
