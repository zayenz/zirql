+++
schema_version = 1
id = "base-036"
key = "report-ordering"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-031"]
+++
# Sort constructed reports and select optional extrema

## Outcome

Both consumers can stably sort constructed values/records and select min/max with optional empty results and first-input ties.

## Context

Read the selected sort/min/max contract. Reuse value comparison, nominal declaration order, grouping’s field selector and the optional runtime; native host sorting remains distinct. Start in src/value.rs/src/reader.rs and constructed report lowering in both consumers.

## Boundaries

- No arbitrary user comparators, automatic absent-key order, implicit coercion or duplicate host-native sorting implementation.

## Done when

- [ ] sort/sort_by and min/max/min_by/max_by implement selected scalar/key types, ascending or descending stable sorting, optional empty extrema and first-input ties.
- [ ] Missing/absent/incompatible keys fail with locations; projected explicit defaults work. Original records and nominal identity are retained.
- [ ] Extract keys once per row; use O(n log n) sorting comparisons and a single extrema scan. Scratch allocation, key inspection and fallible budget charges are explicit.
- [ ] Both consumers expose the shared operations with executable report examples and separate pinned dependency/consumer commits.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Use one repeated-key report plus focused empty, absent-key, incompatible-type and budget controls; measure scaling without timing assertions.
