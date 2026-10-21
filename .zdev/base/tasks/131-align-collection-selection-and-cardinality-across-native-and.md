+++
schema_version = 1
id = "base-131"
key = "coherent-collection-selection"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-125"]
+++
# Align collection selection and cardinality across native and value paths

## Outcome

Filter, count, head, tail, reverse and check share one public collection contract and behave consistently through text, typed builders and host adapters.

## Context

Read the brief's paired-operation coherence decision. Start at src/composition.rs collection_value_types/run_collection/Filter, src/reader.rs ValueSuffixKind/lower_value_suffix/value_suffix and the existing Stage cardinality helpers in src/eval.rs. The value reader exposes filter/count but refuses head/tail/reverse/check; typed common stages can admit those constructed kinds without equivalent value type flow. Value count is an array reducer while native dictionaries can be counted. Native map bounds preserve keys, but ordinary constructed bounds are unavailable. Zincite program/native.rs Shape::Array strips declarations when rebuilding a selection; Zirium query/report.rs selection_items/restore_selection uses separate legacy JSON and declared constructed paths. Close these specific gaps through the existing evaluator.

## Boundaries

- The fixed operation list is filter, count, head, tail, reverse and check. Support ordinary positional arrays, insertion-ordered sets and dictionaries, and admitted native selections/maps; do not add a universal iterable protocol or rewrite map/flat_map/grouping/path algorithms.
- Keyed arrays keep their ordered axes, missing cells and coordinates. Existing count/length counts present cells; other operations require explicit positional/entry conversion. Tuples and scalar text do not become implicit sequences.
- Keep native truth/completeness/Unknown, ownership, source edits and effect admission with the host. Zincite's closed Data policy does not acquire report-only stages.
- Reuse common stage/value preparation and existing cardinality/movement operations. No host legacy flags or alternate evaluation route. Docs change with this slice.

## Done when

- [ ] Filter tests each array/set element or dictionary value in current order, retains original values/keys and collection kind, and preserves duplicates where the input kind permits them. Native predicates keep their existing domain subjects and Unknown/completeness rules; constructed predicates still require known Boolean truth.
- [ ] Count returns collection cardinality: array/set elements, dictionary entries or native stream subjects, with empty count zero. Native exact count results may keep their native scalar representation; explicit constructed conversion remains checked signed conversion.
- [ ] Head/tail accept nonnegative integer expressions, allow zero and oversized bounds, and retain the first/last entries in current order. Reverse reverses all entries including equal-key ties. Each preserves ordinary collection kind and native owners.
- [ ] Check passes its unchanged input through on success, requiring nonempty input without an expected count or exactly the supplied nonnegative count, and retaining the existing optional failure message. check(0) succeeds on empty input for every admitted ordinary collection.
- [ ] Text and callable/typed forms use the same preparation, kind admission, runtime operation and argument semantics. Known invalid kinds/effects/argument types still fail on empty input; later errors leave no partial emitted output.
- [ ] Removal/reordering retains declared element/key/value contracts and typed empty schemas. Structural record field facts narrow consistently when entries are removed instead of retaining invalid required-field claims; no empty schema is guessed from surviving rows.
- [ ] Update the combined collection/pipeline reference, native embedding/API docs and affected Zincite/Zirium guides with signatures, dictionary predicate subjects and the explicit keyed-array boundary; remove duplicated or contradictory native-only descriptions.

## Validation

- Reuse existing filter, bounds, cardinality, schema and buffering controls. Add only missing public boundary controls: one textual/typed constructed chain using a runtime bound, a dictionary operation retaining keys/order, and an empty declared element-schema case that still refuses an invalid field.
- Run the brief's required Rust checks in each changed repository and affected host/Python checks. Run affected documented examples and docs content/link checks; git diff --check and zdev check base --format json for records. Retain separate framework/consumer commits and reviewed Zirql dependency pins.
