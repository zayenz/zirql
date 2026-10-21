+++
schema_version = 1
id = "base-132"
key = "coherent-unique-tally"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-125", "base-131"]
+++
# Share structural unique and tally semantics across value and native collections

## Outcome

Unique and tally use existing structural identity with exact native leaves, retain first occurrences, and compose consistently through supported text and typed collection paths.

## Context

Read the brief's paired-operation coherence decision. src/value.rs unique_elements/unique_value is still homogeneous-scalar-only while tally_value and general sets already use structural hashing/equality. src/native_collections.rs already supplies native_value_hash/native_values_equal/construct_native_set/tally_native_values, and src/functions.rs has a native positional tally route. Core composition still funnels native tally through string-only collection_tally_key and primitive NativeValue::Tally. Zincite pipeline.rs refuses unique over literal-value streams and tally outside strings; Zirium report.rs restricts unique to a few stream kinds. Zirium query/diff_program.rs also carries reject_string_unique, separating typed and textual policies. Both consumers lack the general dictionary view/construction hooks required to retain native keys. Reuse these concrete seams instead of adding new identity rules.

## Boundaries

- The fixed operation list is unique and tally over positional value/native sequences and existing native streams. Sets are already unique and can retain their kind when accepted; dictionaries/keyed arrays do not acquire implicit element iteration merely to widen the overload list.
- Constructed keys use structural identity; native leaves use exact host identity including owner/source/diff side. Preserve nominal admission, floating signed-zero equality and distinct constructed kinds; never deduplicate or tally by rendered text.
- Add only the concrete general dictionary storage/view/construction needed in the two consumers and their existing renderers. Keep keys as actual values until explicit rendering; reuse the shared tagged-entry representation for non-string keys. Do not build a second native container framework.
- The small parse_pipeline API may retain its terminal scalar/tally representation. Full common expressions/programs must compose according to actual result kind, without a blanket tally-terminal rule or frontend compatibility flag. Docs change with this slice.

## Done when

- [x] Constructed unique accepts admitted mixed scalars, absence, records, tuples and nested collections under the same structural identity already used by general sets/tally. It keeps the first original representative and incoming order, with retained declared element contracts on empty output.
- [x] Native/value unique handles admitted positional sequences and existing native stream subjects through the same equality/hash contract, retaining native owners and source/diff identity. Textual and typed builders accept the same supported data; remove reject_string_unique and accidental literal/scalar-container restrictions.
- [x] Tally accepts the same admitted positional/native values as keys, counts structurally or by exact native leaf identity, retains actual first key representatives and first-seen key order, and returns an ordered dictionary with checked counts. Empty tally remains a dictionary with available declared key/value schema.
- [x] Both hosts retain general/native tally keys and values through copies, saved results and lookup, with explicit rendering using their established native leaf projection and shared dictionary encoding. String-key tallies keep ordinary dictionary output.
- [x] Full value/program pipelines may apply dictionary operations such as reverse/head/check/sort_by(value) to a tally result under the preceding collection task's contracts. Scalar count results refuse inappropriate collection continuation by kind; the primitive small reader remains explicitly documented.
- [x] Update unique/tally reference entries together for native/value input, library/API embedding docs, and affected host guides/examples. Explain structural identity versus explicit domain projection without duplicating operation contracts.

## Validation

- Reuse general-set/dictionary/tally/native identity suites. Add a compact public control combining nested/mixed duplicate values, first representations and a tally continuation; add one native control distinguishing equal names from different owner/side identities and one textual/typed unique acceptance case.
- Run the brief's required Rust checks in each changed repository and affected host/Python checks. Run affected documented examples and docs content/link checks; git diff --check and zdev check base --format json for records. Retain separate framework/consumer commits and reviewed Zirql dependency pins.

## Result

Shared structural unique/tally retain first representatives, native owners/source/diff-side identities, actual dictionary keys and declared empty schemas through copies, typed functions, saved lookup and dictionary continuation. Shared rendering handles actual keys; diff execution reuses PreparedProgram. Replaced host string-only tally/unique restrictions and duplicate diff scope execution. Domain operations, source/edit admission, completeness and emission remain host-owned. Consumer commits Zincite8c58749cd7e5774a0b8750f75af993356e261227 and Zirium93225eb36318e127543ba8bf15e7b50673efb818, canonical Zirql pin49c30dac58ed2614fb9d6ba97dafe1432fa7a9cc.

Validation:

- Independent full review, all3findings independently reproduced, correction1 and fresh focused PASS atW465bb430417009d4; exact core and consumer4fields equal.
- All3Rustfmt/Clippy/fullsuites pass; Ziriumalltargets/allfeatures/doctests/warningsdocs; rebuiltPython/Ruff/ty181passed2existingskips; examples/sites6434/1248/897local and253crosslinks; existingquery_profile smoke pass.
