+++
schema_version = 1
id = "base-105"
key = "composed-native-paths"
area = "base"
status = "done"
complexity = "advanced"
priority = "normal"
blocked_by = []
+++
# Design composed relationship paths and intermediate captures

## Outcome

Choose a small path-expression extension grounded in declaration and SSA navigation.

## Context

Current paths follow one registered edge and return one shortest witness per source/sink pair with max_depth. Read src/paths.rs, docs/native-programs.md and the two host path guides. The brainstorm proposed sequencing, alternation, bounded repetition and intermediate captures; none of those spellings is settled. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Design and documentation only; discuss the proposal with the user and record the selected scope before importing implementation follow-ups. Do not edit runtime or consumer code.

## Done when

- [x] Work through one Zirium definition/structure journey and one Zincite reference journey, comparing composed paths with existing pipelines and loops; identify which intermediate capture is useful.
- [x] Present a preferred minimal syntax and alternative, defining edge-kind transitions, hop bounds, shortest-witness/tie rules, cycles, capture ambiguity and incomplete edges while preserving native ownership and diff-side restrictions.
- [x] The selected first slice identifies host adapter needs and a focused witness/refusal example; explicitly defer any unsupported capture or relation operator rather than promising a full graph language.
- [x] The brief and a focused .zdev/_notes/ design document record examples, the user decision, unresolved/deferred options, affected reader/API/host documentation, and a bounded proposed implementation split.

## Validation

- Review the worked examples and alternative against current source and host guides; distinguish proposed syntax from runnable examples.
- Record the user decision; no new product tests are expected.
- git diff --check
- zdev check base --format json

## Result

Selected composition and bounded repetition with named captures deferred; recorded host journeys, witness contracts and documentation follow-ups.

Validation:

- Independent verification passed against W5d16efc803618ea5; snapshot unchanged.
- git diff --check and zdev check base --format json passed.
