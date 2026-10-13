+++
schema_version = 1
id = "base-024"
key = "keyed-entry-alignment"
area = "base"
status = "open"
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

- [ ] Both operations participate in all applicable shared value APIs with typed nominal identities, explicit axes, checked allocation and one budget.
- [ ] Exact pairing preserves left-axis/entry order, accepts identical sparse coverage, and rejects different present-key sets or axis order/types even when element counts agree; equal empty axes are retained.
- [ ] Zincite exposes the operations through CST lowering and library/inline/file reports, comparing two compatible matrices with an explicit per-cell delta comprehension.
- [ ] A documented short --program report extracts matrix entries or pairs compatible matrices; host rectangularity/serialization/completeness rules remain unchanged.

## Validation

- Use one small integer/nominal multidimensional native control with matching sparse cells and one equal-cardinality/different-key refusal; reuse base-007 coverage for host shape policy and empty axes.
- Run required Zirql/Zincite checks and focused constructed-report tests; no solver or new matrix of shapes.
