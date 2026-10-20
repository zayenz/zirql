+++
schema_version = 1
id = "base-108"
key = "named-query-composition"
area = "base"
status = "open"
complexity = "advanced"
priority = "normal"
blocked_by = []
+++
# Design reusable named pipeline queries

## Outcome

Determine whether existing pure functions can provide concise reusable native pipelines.

## Context

Named pure typed functions and native values already exist, while the brainstorm query declaration was a sketch. Read src/functions.rs, src/program.rs, docs/functions-and-control-flow.md and actual Zincite/Zirium function examples. Do not add a second callable system just to supply different spelling. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Design and documentation only; discuss the proposal with the user and record the selected scope before importing implementation follow-ups. Do not edit runtime or consumer code.

## Done when

- [ ] Write a reusable decision-declaration selection and an expensive-user selection with explicit input/parameter/result contracts; show the best current function equivalent.
- [ ] Compare function reuse with a minimal query declaration/suffix form, resolving current-input binding, lexical scope, purity, argument checking and native kind transitions.
- [ ] Record whether any syntax is needed and the smallest useful saved-definition workflow; imports, package management and general closures remain separate decisions.
- [ ] The brief and a focused .zdev/_notes/ design document record examples, the user decision, unresolved/deferred options, affected reader/API/host documentation, and a bounded proposed implementation split.

## Validation

- Review the worked examples and alternative against current source and host guides; distinguish proposed syntax from runnable examples.
- Record the user decision; no new product tests are expected.
- git diff --check
- zdev check base --format json
