+++
schema_version = 1
id = "base-024"
key = "keyed-entry-alignment"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-018", "base-007"]
+++
# Inspect and exactly align keyed data through shared value operations

## Outcome

Zirql exposes present keyed entries and exact keyed pairing; Zincite uses them for coordinate-preserving data comparison reports.

## Context

Read the keyed research section, background/comprehensions.md, src/value.rs keyed types and completed base-007 consumer contract. Keyed values exist already; IndexSet is rank-one-only. This task adds operations rather than reimplementing construction or MiniZinc validation.

## Boundaries

- entries returns positional records containing key and value in established entry order: a scalar key for rank one, a tuple for higher rank. The source retains its axes.
- zip_exact initially accepts keyed arrays only and returns a keyed array of two-element tuple cells with identical ordered axes and present-key sets, including identical sparse coverage. No truncation, reindexing, filling or data writes.

## Done when

- [x] Both operations participate in all applicable shared value APIs with typed nominal identities, explicit axes, checked allocation and one budget.
- [x] Exact pairing preserves left-axis/entry order, accepts identical sparse coverage, and rejects different present-key sets or axis order/types even when element counts agree; equal empty axes are retained.
- [x] Zincite exposes the operations through CST lowering and library/inline/file reports, comparing two compatible matrices with an explicit per-cell delta comprehension.
- [x] A documented short --program report extracts matrix entries or pairs compatible matrices; host rectangularity/serialization/completeness rules remain unchanged.

## Validation

- Use one small integer/nominal multidimensional native control with matching sparse cells and one equal-cardinality/different-key refusal; reuse base-007 coverage for host shape policy and empty axes.
- Run required Zirql/Zincite checks and focused constructed-report tests; no solver or new matrix of shapes.

## Result

Added shared keyed entries and exact pairing with coordinate-preserving Zincite matrix reports. Reviewed framework API 24cb9d68838d46af63a052119bc0401bbf99272f and Zincite integration 1d82b409363418f48c83e57b1521f4e80cb486c0 are pushed. Zincite uses that immutable Git revision; Zirium remains unchanged.

Validation:

- Independent core and whole-task reviews passed without corrections. All four completion conditions passed, including sparse generic alignment, typed nominal keys, retained empty axes, exact-key refusals and one continuing budget.
- Required core and Zincite formatting, Clippy, workspace tests and doctests passed. The whole review checked native matrix pairs, entries and deltas 1,2,3,4 and twenty direct library/inline/file CLI controls, including source preservation, completeness and later-error buffering.
- Core /zirql and Zincite root and /zincite documentation checks/builds and actual navigation, search, themes and mobile layout passed. Independent fresh Zincite builds matched all 116 reviewed artifacts and checked 1,158 local links/assets; core review matched 54 artifacts and checked 541 links/assets.
- Existing host rectangularity/serialization rules and the closed data-edit call whitelist remain unchanged. The existing empty e.key comprehension limitation is documented; direct zip_exact retains established empty axes.
