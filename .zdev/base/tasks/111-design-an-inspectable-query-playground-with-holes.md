+++
schema_version = 1
id = "base-111"
key = "query-playground-holes"
area = "base"
status = "done"
complexity = "advanced"
priority = "low"
blocked_by = []
+++
# Design an inspectable query playground with holes

## Outcome

Choose a small local exploration tool that previews values and applicable next operations.

## Context

The core has located parsing, type preparation and finite budgets, but no generic interactive playground. Read src/reader.rs, runtime type/host operation contracts and examples/csv.rs. Use a CSV interaction plus a Zirium or Zincite native operation inventory to identify real host requirements. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Design and documentation only; discuss the proposal with the user and record the selected scope before importing implementation follow-ups. Do not edit runtime or consumer code.

## Done when

- [x] Sketch one complete interaction from a pipeline hole to a selected stage and resulting preview, including an invalid or incomplete expression and an empty input.
- [x] Choose a local CLI/REPL or minimal browser presentation and define the smallest host operation inventory needed; distinguish known applicable stages from unestablished types.
- [x] Preview execution is explicitly read-only and bounded, shows sampled/truncated display honestly, never executes edit/emission stages, and does not reinterpret a parse hole as a runnable query. Record documentation and implementation follow-ups for the selected interface.
- [x] The brief and a focused .zdev/_notes/ design document record examples, the user decision, unresolved/deferred options, affected reader/API/host documentation, and a bounded proposed implementation split.

## Validation

- Review the worked examples and alternative against current source and host guides; distinguish proposed syntax from runnable examples.
- Record the user decision; no new product tests are expected.
- git diff --check
- zdev check base --format json

## Result

Selected a local CSV exploration mode with terminal holes outside syntax, explicit bounded read-only previews, honest display truncation and scoped implementation/docs follow-ups.

Validation:

- Independent verification passed against W41ac618eeb18cb00; snapshot unchanged.
- git diff --check and zdev check base --format json passed.
