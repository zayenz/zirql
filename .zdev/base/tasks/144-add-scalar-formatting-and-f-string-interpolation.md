+++
schema_version = 1
id = "base-144"
key = "string-formatting"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Add scalar formatting and f-string interpolation

## Outcome

Reports use f-strings and format(value, spec) to construct text through one scalar formatter that shares the request budget.

## Context

The user requested readable text construction beyond to_string and ++. Read the brief section String formatting and interpolation (2026-10-22), which defines syntax, specifications and scalar boundaries. Start with src/parser.rs protected spans, src/reader.rs value atoms and scan_step, src/value.rs scalar conversion and binary operators, and the prepared/runtime traversal paths. CSV numeric-order labels are the concrete use case.

## Boundaries

- Keep existing plain strings and host-specific emitters unchanged. Do not modify Zincite or Zirium or update their dependency pins.
- Reuse the shared reader, preparation and evaluator; add no template engine, formatting dependency, host reflection or implicit native rendering.

## Done when

- [ ] f-strings and format(value, spec) follow the contract in the brief when used in expressions, functions and CSV reports.
- [ ] Field scope, effects, type preparation, native boundaries, left-to-right evaluation, original source locations and cumulative budget limits follow existing expression rules.
- [ ] The scalar/string reference, language and embedding docs describe supported formats and limitations; a runnable CSV example uses interpolation. All added explanatory text receives the requested Astra medium copyedit.

## Validation

- Run cargo fmt --all -- --check, cargo clippy --offline --locked --workspace --all-targets -- -D warnings, and cargo test --offline --locked --workspace.
- Add focused public behavior checks for fields containing nested syntax, escaped braces, alignment/base/precision formatting, strict scalar refusals and known-type preflight, lazy value failures, source ranges and oversized output budget refusal. Reuse current test helpers.
- Run the documented CSV example, git diff --check and zdev check base --format json. Existing site organization needs no new page or build system.
