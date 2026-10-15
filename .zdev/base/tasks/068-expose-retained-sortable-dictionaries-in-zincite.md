+++
schema_version = 1
id = "base-068"
key = "zincite-native-dictionaries"
area = "base"
status = "open"
complexity = "advanced"
afk = true
blocked_by = ["base-067"]
+++
# Expose retained sortable dictionaries in Zincite

## Outcome

Zincite native dictionary construction, value sorting and bounds preserve insertion/current order through library and JSON results.

## Context

Read background/collection-language-contract.md native dictionary contract and completed core order/typed native comparison tasks. Zincite program.rs Stage/Kind/ReportValue currently lack native map/dictionary stages; report_json assembles default serde_json maps. Reuse core map_native_by, typed value comparison and bounds, current singleton native adapters and existing Record representation where value/shape contracts fit. map_by requires exactly one string key per item and retains arbitrary already-admitted pure native/constructed payloads.

## Boundaries

- No separate ranked-map result kind, compatibility adapter, implicit entries(dictionary) helper, arbitrary map key types, reflection or broader data-edit/emit semantics.

## Done when

- [ ] map_by(key_query,value_query) evaluates each key then its value against the same singleton in input order, rejects duplicates before their values and retains native payload owners/limitations. Empty input is an empty dictionary.
- [ ] Dictionary sort_by(value) uses the accepted typed scalar/nominal contract, is stable in current order and returns the same dictionary kind. reverse/head/tail, saved bindings, copies and nesting preserve that kind and order, including empty/zero/oversized bounds.
- [ ] JSON object order agrees with native/library iteration, including nested and constructed dictionaries, within the existing complete/limitations/result envelope. Use an existing order-preserving JSON option or explicit serializer at the rendering boundary; never implicitly output key/value record lists.
- [ ] Native tally and the already shared constructed tally expose first-seen dictionary keys without recreating counts. Known unsupported kinds/effects fail preparation even when empty, and a later failure leaves CLI stdout empty.
- [ ] Focused direct/inline/file/library controls cover declaration-name/use-count dictionary sorting, reverse then stable reranking of ties, numeric/nominal payload keys, duplicate-before-value refusal, nested JSON and empty dictionaries. Use a real pinned shared Git dependency and separate consumer/core commits where needed.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.
