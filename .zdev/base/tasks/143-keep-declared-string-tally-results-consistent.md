+++
schema_version = 1
id = "base-143"
key = "declared-string-tally"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Keep declared-string tally results consistent

## Outcome

Tallying equivalent ordinary strings produces the same string dictionary for literals and typed function calls.

## Context

An independent check reproduced different JSON shapes for `fn echo(s:string)->string{s} report [echo("a"),echo("a")] | tally | json` and a literal string tally in Zirium. In core src/value.rs, tally_value forces Dictionary storage for any declared key; finish_dictionary otherwise normalizes string keys to Record. General dictionaries deliberately render entry pairs. Preserve that codec and correct tally construction. Existing string-dictionary construction already handles declared string keys. Start with tally_value, finish_dictionary and the existing structural-key and empty-schema controls.

## Boundaries

- Preserve first-seen order, counts, admission, budgets, declared empty schemas and required outer declarations.
- Keep non-string and general keys and their representative values intact. Do not change explicit tagged rendering or the general-dictionary entry-pair codec.

## Done when

- [x] Literal and typed-function string tallies have matching dictionary behavior and ordinary JSON output.
- [x] General structural keys and declared empty results retain their existing behavior.
- [x] Update the tally/API reference and affected embedding or host guidance with one useful example. Added explanatory prose receives Astra medium copyediting with unslop and zayenz-writing-style.

## Validation

- Add one focused public core control for declared string keys; reuse structural-key and empty-schema controls.
- Exercise the equivalent typed-function string tally through CSV, Zincite and Zirium against the same reviewed core revision.
- Run required Rust checks in the affected repositories and git diff --check. Use the existing offline dependency cache; do not add a benchmark or test matrix.

## Result

Literal and typed-function string tallies produce matching dictionaries and ordinary output. Tally results retain declared dictionary contracts, including empty results; general dictionary rendering is unchanged.

Validation:

- Required offline locked Rust formatting, Clippy and tests passed in affected repositories; Zirium doctests and warnings-denied docs passed. Focused public controls passed.
- Astra medium reviewed added prose using unslop and zayenz-writing-style. Both consumers use canonical Zirql dd5927fb0e7ecea1a96bf0f9b43efbe951d64048.
- Independent verifier passed; exact core snapshot and consumer states matched before completion.
