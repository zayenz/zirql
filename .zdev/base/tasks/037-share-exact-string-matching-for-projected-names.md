+++
schema_version = 1
id = "base-037"
key = "shared-string-helpers"
area = "base"
status = "done"
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

- [x] The three operations implement exact case-sensitive matching, including true for empty search strings, and reject non-string operands with located errors.
- [x] Scanning and failure diagnostics spend the active budget without unnecessary substring or normalization copies.
- [x] Both hosts demonstrate projected-name filtering through inline/file/library paths and document the helpers; record separate integration commits and Git pins.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Add a compact shared control covering Unicode text, empty search and a type/budget refusal; reuse it for host acceptance.

## Result

Shared exact string matching is available in both consumers through text and library APIs. The core borrows retained strings, charges scans and diagnostic work, and preserves located refusals. Zincite integration bb3525b0d2f5de02313e07311d782a664e46179e and Zirium integration b22f224afb91355927ee7315c489ebeff37d1915 use core Git revision 8acf1cac99225407b7cda17789108312a3519ee3.

Validation:

- Fresh independent whole-task review passed with no required corrections; the stored work context and both external consumer Git/source guards remained equal.
- All 47 required Rust, record, Python and site commands passed; CPython 3.11 through 3.14 each passed 172 tests with two skips.
- Focused Unicode, empty-search, type and scan-budget controls pass through inline, file, library and typed paths; later runtime failures leave CLI stdout empty. Zincite data-edit exclusions remain intact.
- Actual Git manifests, lockfiles and fetched source match the accepted core; separately attributable dependency and implementation commits are recorded in both consumers.
- Root reviewed the combined core and consumer guides at production bases in light/dark desktop/mobile layouts. Search, navigation, links, anchors, canonical paths, assets and console checks passed; all 219 documentation/output hashes match independent rebuilds.
