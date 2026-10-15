+++
schema_version = 1
id = "base-062"
key = "ordered-collections-runtime"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = []
+++
# Preserve collection insertion order and collection kind when sorting

## Outcome

Callable shared set and dictionary construction, sorting and tally preserve insertion/current order without changing their collection kind.

## Context

Read the brief Collection syntax and order decision, docs/values.md selected collection section, and docs/native-programs.md accepted alignment. Current src/value.rs insert_set/check_set normalize scalar sets; equal_values zips them; constructed sorting requires arrays and tally sorts string fields. Record already stores ordered string fields and compares associations. src/construction.rs construct_entries and src/mapping.rs map_native_by return BTreeMap, while ranking uses lexical-key ties. Change these concrete shared paths once; existing native algorithms and Record/Set storage are starting points. This library slice supports the subsequent consumer migrations, which remain pinned to their admitted older dependency until migrated.

## Boundaries

- No collection hierarchy, general comparator callbacks, common model resolver, string-set expansion, arbitrary-key mapping type or new data-edit permission. Consumer migration is separate; do not edit consumer repositories in this task.

## Done when

- [x] Existing Set construction, admission, copies and membership equality preserve the first representative of duplicates without sorting automatically; explicit sort retains Set and its available type metadata. Nominal/type checks and finite numeric identity remain enforced.
- [x] Static and computed string dictionaries preserve insertion order using existing string-field values wherever adequate. Key evaluation, duplicate check and value evaluation retain their order, with duplicate refusal before that value. No generalized arbitrary-key dictionary or separate everyday record syntax is introduced.
- [x] Shared native dictionary construction returns ordered entries, stable value sorting keeps current-order ties, and reverse/head/tail retain dictionary semantics. Remove the lexical/ranked result distinction where it is only compatibility machinery; preserve retained native payloads and active accounting.
- [x] Constructed sort supports dictionary keys, the closed value selector and existing static value-field selector, retaining a dictionary; descending comparison preserves ties. Tally counts remain unchanged and keys follow first occurrence. Dictionary association equality remains independent of iteration order.
- [x] Compact public controls demonstrate unsorted set insertion/deduplication and equality after sorting, dictionary z:2/a:2/m:1 insertion and stable reranking, duplicate-before-value refusal, first-seen tally and empty same-kind results. Existing resource/type/location checks remain meaningful.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.

## Result

Shared sets and dictionaries preserve insertion order; sorting retains collection kind and current ties; set equality compares membership and both tally paths retain first-seen keys. Ordered native construction retains payload ownership.

Validation:

- Independent whole-task review passed without corrections; exact native snapshot and owned/foreign guards matched before completion.
- Canonical formatting, Clippy with warnings denied, 125 integration tests and 8 doctests passed; compact controls cover order, same-kind empties, equality, stable ties, failure precedence, nominal metadata and limits.
- Five runnable word examples confirm filtered/all-word first-seen tally and dictionary key/value/field sorting; homepage semantic changes are separately staged while preserving unrelated prose.
- Site check/build passed; 19 pages and 910 internal links have no errors; desktop/mobile guides and homepage tally inspected in the rendered site.
- Accepted future grammar/native contracts retained in zdev; consumers remain pinned until their separate migrations, with no publication.
