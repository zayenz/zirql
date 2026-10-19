+++
schema_version = 1
id = "base-100"
key = "string-set-tally"
area = "base"
status = "open"
complexity = "standard"
afk = true
blocked_by = []
+++
# Tally unordered string sets in constructed word queries

## Outcome

The shared language constructs string sets and tallies arrays of them by unordered membership, with a runnable neighbouring-word example.

## Context

The blog author wants [{words[i], words[i + 1]} | i in index_set(words) where i < length(words)] | tally for words ash apple birch apple, producing one ash/apple pair and two apple/birch pairs. Current SetElementType excludes strings and constructed tally accepts only strings. Begin with src/value.rs, src/value/execution.rs, src/reader.rs, tests/value_expressions.rs, and examples/words.rs. Use a small ordinary constructed result shape for non-string tally keys rather than encoding sets into strings or generalizing dictionaries. The explicit feature request authorizes this task creation, import and implementation.

## Boundaries

- Keep existing string tally dictionary behavior where practical; preserve insertion order of sets and first-seen order of tally groups, using unordered membership for equality.
- Keep the change in Zirql; do not edit consumers or the blog, publish a release, deploy, or broaden arbitrary dictionary-key support. Preserve unrelated local work.

## Done when

- [ ] String set literals, set comprehensions, typed empty sets and their relevant shared type/equality/ordering/serialization paths work coherently with the existing budget and located-error contracts.
- [ ] Tally accepts arrays of homogeneous string sets, groups reversed insertion orders together and collapses equal adjacent words to singleton keys; its result representation and empty-result behavior are documented, and string tally behavior is preserved.
- [ ] Focused public-behavior checks cover empty and singleton word inputs, reverse-order equality, repeated identical words, result order, types, and meaningful error/budget behavior without a broad test matrix.
- [ ] The runnable word host and documentation show the exact supported neighbouring-set query and printed output for ash apple birch apple, and all required checks pass.

## Validation

- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- Run the documented words example with the exact query and inspect its printed result.
- git diff --check
- zdev check base --format json
