+++
schema_version = 1
id = "base-109"
key = "native-finding-reports"
area = "base"
status = "done"
complexity = "advanced"
priority = "normal"
blocked_by = []
+++
# Design explanatory finding records for small linters

## Outcome

Choose the smallest way to return multiple useful findings with retained native subjects.

## Context

require fails a calculation; ordinary records and native projection can already retain explanatory values. Read docs/native-programs.md, docs/semantic-models.md and host rendering contracts. Use an unused MiniZinc declaration and a suspicious MLIR conversion as examples without claiming domain facts the hosts cannot establish. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Design and documentation only; discuss the proposal with the user and record the selected scope before importing implementation follow-ups. Do not edit runtime or consumer code.

## Done when

- [x] Compare ordinary record conventions against a dedicated finding helper using message, native subject and optional details; prefer a convention if it meets both examples.
- [x] Define how host rendering locates the subject, how incomplete facts are reported, and how multiple findings remain distinct from runtime failures and budget exhaustion.
- [x] Record one chosen local demonstration path and any explicitly scoped host rendering follow-ups, with documentation showing actual supported output rather than promising editor integration.
- [x] The brief and a focused .zdev/_notes/ design document record examples, the user decision, unresolved/deferred options, affected reader/API/host documentation, and a bounded proposed implementation split.

## Validation

- Review the worked examples and alternative against current source and host guides; distinguish proposed syntax from runnable examples.
- Record the user decision; no new product tests are expected.
- git diff --check
- zdev check base --format json

## Result

Selected ordinary native finding records; documented domain evidence, local demonstration, rendering and error boundaries, and documentation follow-ups.

Validation:

- Independent verification passed against W0ffb125d136da77c; snapshot unchanged.
- git diff --check and zdev check base --format json passed.
