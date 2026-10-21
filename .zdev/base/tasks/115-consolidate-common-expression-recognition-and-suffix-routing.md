+++
schema_version = 1
id = "base-115"
key = "common-reader-routing"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-113"]
+++
# Consolidate common expression recognition and suffix routing

## Outcome

Consumers rely on the Zirql reader for shared syntax decisions instead of duplicating scanners and common-form lists.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Zirql src/reader.rs ordering_call duplicates Zirium query/comprehension.rs ordering_call; surrounding collection_syntax/has_syntax/starts and parser.rs starts_native route shared syntax. Zincite program/functions.rs common_suffix/constructed_call/take_constructed_suffix and program/composition.rs bridge common expressions. Zirium query.rs parse_with_initial_bindings also uses recognition to account parsing work.

## Boundaries

- Preserve deliberate host lexical forms, legacy JSON/interpolation and MiniZinc edit grammar. Do not introduce a configurable lexer/parser framework.

## Done when

- [ ] A bounded common ordering-call probe replaces Zirium’s copy, preserving consumed positions, nested delimiters, comments, locations and budget behavior.
- [ ] Common expression-start and suffix classification used by both hosts moves into existing reader APIs; remove replaced host grammar lists/scanners where their decision is wholly shared. Retain only documented native-stage/legacy-literal/effect distinctions.
- [ ] Constructed and native calls with overlapping names, comments/strings and saved-result suffixes follow one common grammar decision; legacy parsing limits are preserved or explicitly reconciled in docs rather than accidentally bypassed.
- [ ] Embedding guidance explains the smaller reader adapter and host docs reflect support.

## Validation

- Use existing parsing tests plus a compact ambiguous ordering/suffix example and located malformed-input/limit check.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
