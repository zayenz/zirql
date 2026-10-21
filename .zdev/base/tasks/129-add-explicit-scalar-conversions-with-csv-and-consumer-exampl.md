+++
schema_version = 1
id = "base-129"
key = "scalar-text-conversions"
area = "base"
status = "done"
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

- [x] Textual and callable parse_int(string), parse_float(string) and to_string(bool|int|float|string) implement the brief’s grammar, overflow/underflow, formatting and located-error contract, including transparent declared wrappers.
- [x] Preparation/type discovery rejects incompatible known operands, retains normal deferred admission and validates empty/skipped contexts consistently with existing operations; runtime conversion remains lazy in unchosen branches.
- [x] Both consumers expose the operations in ordinary reports/functions without host-specific conversion helpers; native ownership, closed edit restrictions and existing common operation behavior are preserved.
- [x] CSV documentation demonstrates numeric filtering and a numeric summary over unchanged string columns, plus labels using to_string; examples show malformed/empty input errors and make explicit that whitespace is not trimmed.
- [x] Rust API docs, the library reference’s scalar/string category, alphabetical index, sidebar and current highlighting sources include the three calls, using the canonical consumer-sync procedure if base-122 has landed. Host guides link the contract and each include or update one useful report recipe.

## Validation

- Add focused shared checks for signed integer boundaries, decimal/exponent float parsing, nonfinite/rounded-to-zero refusal, locale-independent scalar formatting and a skipped runtime failure; combine cases in existing value tests without a syntax matrix. Run the CSV numeric recipe and a representative ordinary report in each host.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.

## Result

Added parse_int, parse_float and to_string through the existing unary reader, type discovery and evaluator with exact numeric grammar, finite/underflow/signed-zero rules and active budgets. Zincite retains whole-program closed Data refusals; Zirium Rust/Python builders forward to shared operations. Added numeric CSV fixture/recipe, host recipes and reference/index/sidebar docs. Zincite 5ee2f566be575c29ea33a5d0e52ab04cf6a1ca0f; Zirium e77a0f9e137d6f3d60fdef5d80f1524ab78cc68f; canonical Zirql 402eede8a28008b52a36afa1fd301b3bab251e0a.

Validation:

- Independent full review PASS W4e04abe484ba0bd4; unchanged core and complete consumer states. Rust formatting, fatal Clippy, workspace tests and sites passed all three repositories, including Zirium required all-feature/doc checks. Rebuilt Python, Ruff/ty and 18 affected query tests passed. CSV quantity12, two25.0 totals and labels verified; malformed/empty cells fail with locations and empty stdout. Host CLI/library/Python recipes, conversion anchors and unchanged generic highlighting checked.
