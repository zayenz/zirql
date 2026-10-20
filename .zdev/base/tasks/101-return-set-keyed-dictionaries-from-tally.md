+++
schema_version = 1
id = "base-101"
key = "set-key-dictionaries"
area = "base"
status = "open"
complexity = "standard"
afk = true
blocked_by = []
+++
# Return set-keyed dictionaries from tally

## Outcome

Dictionary literals, comprehensions and tally retain constructed values as keys, including tuples, arrays, sets and dictionaries, with structural equality and no string encoding.

## Context

The user's correction supersedes base-100's record-array workaround and the older string-only dictionary restriction. Keep existing string dictionary behavior; add a distinct constructed dictionary representation for general constructed keys, shared preparation, lookup and association equality. Begin at src/value.rs and src/value/execution.rs, the collection contract, and the existing words test/example. The later user clarification explicitly prefers all constructed collections as dictionary keys and values. The explicit request authorizes task creation, import and implementation.

## Boundaries

- Keep this change within Zirql; audit consumer serialization behavior and document integration requirements without editing consumers or the blog.
- Do not publish or change the release version. Keep string tally results compatible and JSON rendering host-owned.

## Done when

- [ ] General constructed dictionary keys work in literals/comprehensions, and set tally returns a dictionary, preserving first-seen order, structural key identity and duplicate refusal before value evaluation.
- [ ] Lookup, known type checks, copy/admission, equality, value sorting and supported display paths handle set dictionaries coherently, with explicit unsupported boundary errors where needed.
- [ ] Focused public behavior verifies the exact word query, reversed/equivalent sets, singleton/duplicate members, typed empty input and existing string dictionaries; documentation distinguishes language values from JSON.

## Validation

- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- Run documented exact words query and inspect output.
- git diff --check; zdev check base --format json
