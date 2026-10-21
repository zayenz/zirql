+++
schema_version = 1
id = "base-129"
key = "scalar-text-conversions"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-115"]
+++
# Add explicit scalar conversions with CSV and consumer examples

## Outcome

parse_int, parse_float and to_string provide common bounded scalar conversions through textual and callable Zirql in CSV and both consumers.

## Context

Read brief.md sections Consolidate language machinery and data rendering (2026-10-21) and Complete the common language boundary (2026-10-21), including their compatibility, documentation and validation contracts. Read the exact scalar conversion contract in the new brief section before editing. CSV decode_table preserves string cells; src/value.rs owns finite/exact numeric behavior and src/construction.rs bounded strings. Extend the existing common reader, prepared evaluator, type discovery and traversal, reusing base-115 name admission. Consumers now use the accepted extensions revision; migrate only affected integration matches/builders and policies.

## Boundaries

- Implement these three conversions only. No automatic cell inference, numeric coercion, trim/regex/format templates, native handle stringification or widening of Zincite’s closed data-edit subset.
- Conversions are pure and use the active budget. Native data must first use its existing scalar projection.

## Done when

- [ ] Textual and callable parse_int(string), parse_float(string) and to_string(bool|int|float|string) implement the brief’s grammar, overflow/underflow, formatting and located-error contract, including transparent declared wrappers.
- [ ] Preparation/type discovery rejects incompatible known operands, retains normal deferred admission and validates empty/skipped contexts consistently with existing operations; runtime conversion remains lazy in unchosen branches.
- [ ] Both consumers expose the operations in ordinary reports/functions without host-specific conversion helpers; native ownership, closed edit restrictions and existing common operation behavior are preserved.
- [ ] CSV documentation demonstrates numeric filtering and a numeric summary over unchanged string columns, plus labels using to_string; examples show malformed/empty input errors and make explicit that whitespace is not trimmed.
- [ ] Rust API docs, the library reference’s scalar/string category, alphabetical index, sidebar and current highlighting sources include the three calls, using the canonical consumer-sync procedure if base-122 has landed. Host guides link the contract and each include or update one useful report recipe.

## Validation

- Add focused shared checks for signed integer boundaries, decimal/exponent float parsing, nonfinite/rounded-to-zero refusal, locale-independent scalar formatting and a skipped runtime failure; combine cases in existing value tests without a syntax matrix. Run the CSV numeric recipe and a representative ordinary report in each host.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.
