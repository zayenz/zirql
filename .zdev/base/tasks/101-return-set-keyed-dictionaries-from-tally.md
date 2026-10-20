+++
schema_version = 1
id = "base-101"
key = "set-key-dictionaries"
area = "base"
status = "done"
complexity = "standard"
afk = true
blocked_by = []
+++
# Return set-keyed dictionaries from tally

## Outcome

Sets, dictionary literals/comprehensions and tally retain ordinary values, including nested collections and host-native handles. Structural equality, hashing and ordering preserve value kinds without string encoding.

## Context

The user's correction supersedes base-100's record-array workaround and the older string-only dictionary restriction. Keep existing string dictionary behavior; add a distinct constructed dictionary representation for general constructed keys, shared preparation, lookup and association equality. Begin at src/value.rs and src/value/execution.rs, the collection contract, and the existing words test/example. The later user clarification explicitly prefers all constructed collections as dictionary keys and values. The user additionally requests sets of arbitrary constructed values, general hashing/comparison, indexed lookup without prior-key scans, and retained domain handles from different sources/query paths. The explicit request authorizes task creation, import and implementation.

## Boundaries

- Keep this change within Zirql; audit consumer serialization behavior and document integration requirements without editing consumers or the blog.
- Do not publish or change the release version. Keep string tally results compatible and JSON rendering host-owned.

## Done when

- [x] General sets (including mixed/nested values) and dictionary keys work in literals/comprehensions, and tally returns a dictionary, preserving first-seen order, structural identity and duplicate refusal before value evaluation.
- [x] Public constructed-value hash/comparison, structural collection sorting, known type checks, copy/admission and display handle general values coherently. Construction/tally use hash collision buckets; repeated general-dictionary lookup uses a cached index.
- [x] Native equality/hash/comparison and FunctionHost collection views/constructors retain domain handles from different query paths/sources under existing identity, ownership and stale-handle rules. Host-native cross-kind ordering is explicit.
- [x] Focused public behavior verifies the exact word query, reversed/equivalent sets, singleton/duplicate members, typed empty input, structural hashing/comparison, native retention and existing string dictionaries; documentation distinguishes language values from JSON.

## Validation

- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- Run documented exact words query and inspect output.
- git diff --check; zdev check base --format json

## Result

General constructed and native values now participate in sets and dictionaries with structural comparison, hashing and indexed operations; tally returns actual keyed dictionaries.

Validation:

- Independent full review and focused rechecks passed; final snapshot W67b9c4ad8d7bd0f6 unchanged.
- cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace: 165 tests passed.
- Exact adjacent-word set tally returns set keys with counts 1 and 2; declared empty string dictionary compatibility verified.
- git diff --check; zdev check base --format json passed.
