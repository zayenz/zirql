+++
schema_version = 1
id = "base-107"
key = "full-value-alignment"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Align two record sequences by unique keys

## Outcome

Ordinary reports can compare matched and unmatched rows from two inputs.

## Context

zip_exact currently demands matching axes and present cells; native semantic diff belongs to Zirium. Begin at existing keyed helpers, structural key comparison and reader execution. Main base-101 already supports general dictionary keys. Use two projected model summaries or benchmark tables as the concrete comparison. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- No new semantic diff engine, multi-version history service, recursive differences helper, implicit cross-source native identity matching or join mode family. Defer expression selectors until a concrete need.
- Keep this slice in Zirql and its local examples; record concrete consumer adoption requirements without changing consumer repositories.
- No Gecode dependency, symbolic CP lowering, first-class function system, publication or deployment.

## Done when

- [x] Provide shared textual and callable align(left, right, by="field") with full alignment and unique present keys on each side; use existing structural key identity and reject duplicates rather than multiplying rows.
- [x] Results contain key, before and after, with absence for an unmatched side. Emit keys in left input order followed by right-only keys in right input order; preserve records and native values without serialization.
- [x] A runnable comparison handles missing sides explicitly before field access. Focused checks cover matched/left-only/right-only rows, empty sides, duplicate keys and equal structural keys with different insertion order.
- [x] Update the relevant public language guide and callable/embedding documentation with the implemented contract, supported host/example entry point, limitations and a runnable example with expected output. Document necessary consumer adoption separately.

## Validation

- Use the focused behaviour checks described in Done when; run the documented local example and compare its result.
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- git diff --check
- zdev check base --format json

## Result

Implemented full unique-key alignment with structural identity, stable order, ordinary optional side values and retained native rows/contracts. Added benchmark comparison and public/API/adoption documentation. Independent whole review and focused accounting recheck passed at W0dfe497e2f6d08ad.

Validation:

- Formatting, fatal workspace Clippy, 179 workspace tests and 6 doctests, diff whitespace and zdev consistency checks passed.
- Focused structural/duplicate/empty/absence/budget/native controls passed and documented comparison matched exact JSON. Corrected two-operand allocation admission; focused recheck passed and final snapshot comparison equal.
