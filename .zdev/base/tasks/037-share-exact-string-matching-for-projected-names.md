+++
schema_version = 1
id = "base-037"
key = "shared-string-helpers"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Share exact string matching for projected names

## Outcome

Both query languages use the same contains, starts_with and ends_with semantics on constructed/projected strings.

## Context

Read the selected string contract; start at scalar operations in src/value.rs, common expression reading in src/reader.rs and each host’s expression lowering. Keep domain name extraction with the host.

## Boundaries

- No regex, case folding, Unicode normalization or dependency on user-defined functions.

## Done when

- [ ] The three operations implement exact case-sensitive matching, including true for empty search strings, and reject non-string operands with located errors.
- [ ] Scanning and failure diagnostics spend the active budget without unnecessary substring or normalization copies.
- [ ] Both hosts demonstrate projected-name filtering through inline/file/library paths and document the helpers; record separate integration commits and Git pins.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Add a compact shared control covering Unicode text, empty search and a type/budget refusal; reuse it for host acceptance.
