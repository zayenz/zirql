+++
schema_version = 1
id = "base-036"
key = "report-ordering"
area = "base"
status = "done"
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

- [x] sort/sort_by and min/max/min_by/max_by implement selected scalar/key types, ascending or descending stable sorting, optional empty extrema and first-input ties.
- [x] Missing/absent/incompatible keys fail with locations; projected explicit defaults work. Original records and nominal identity are retained.
- [x] Extract keys once per row; use O(n log n) sorting comparisons and a single extrema scan. Scratch allocation, key inspection and fallible budget charges are explicit.
- [x] Both consumers expose the shared operations with executable report examples and separate pinned dependency/consumer commits.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Use one repeated-key report plus focused empty, absent-key, incompatible-type and budget controls; measure scaling without timing assertions.

## Result

Shared stable sorting and optional extrema are available through both consumers, preserving first-input ties, original rows, nominal identity and located refusals. Core API a278c32cace3200d496a3c0649f4cf069996e2ad is integrated by Zincite dd426038ece5825fdbad80517d11a978870ea681 and Zirium fb3d0c516d0dc01b85d2a9d3070107debac6739f. Each consumer commit includes the matching exact Git pin and adapters; separately attributable source dependency and implementation commits preserve the pin-before-API sequence.

Validation:

- Fresh independent whole-task review passed without required corrections. The exact stored work context, both consumer candidates and canonical Python files remained unchanged.
- All 47 required Rust, record, Python and site checks passed. CPython 3.11 through 3.14 each passed 173 tests with two existing macOS skips.
- Public repeated-key reports agree across inline, file, library and typed APIs. Focused empty/default, nominal, missing/absent/incompatible key, resource and prepared native-argument controls pass; later CLI failures leave stdout empty. Native host ordering and Zincite Data policy remain intact.
- Ninety temporary release samples retain exact original source attribution. At 8192 rows, key selection occurred 8192 times, sorting used 97768 or 97744 comparisons and extrema used 8191. Wide extrema copied 19 nodes and 4379 string bytes versus 155649 nodes and 35872768 bytes for sorting/input. Ordering algorithms were unchanged by sibling reconciliation; instrumentation was removed before final checks. These are algorithmic observations, not a general speedup claim.
- Actual manifest, lockfile and fetched source pins match the accepted core. Root reviewed core and both consumer guides in light/dark desktop/mobile layouts, including search, navigation, links, anchors and production canonical paths. All 247 documentation/output hashes match independent rebuilds.
