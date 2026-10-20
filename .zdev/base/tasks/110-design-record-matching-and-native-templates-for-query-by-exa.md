+++
schema_version = 1
id = "base-110"
key = "query-by-example"
area = "base"
status = "open"
complexity = "advanced"
priority = "normal"
blocked_by = []
+++
# Design record matching and native templates for query by example

## Outcome

Select a small query-by-example experience that reuses existing patterns where possible.

## Context

Existing value patterns can already match literals, wildcards and record shapes. Compare a matching stage against selective generators without making that unimplemented task a prerequisite. Read src/value/pattern.rs, src/matching.rs and host capture documentation. Native syntax holes require host parsing and identity rules. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- Design and documentation only; discuss the proposal with the user and record the selected scope before importing implementation follow-ups. Do not edit runtime or consumer code.

## Done when

- [ ] Work through a record selection and a copied MiniZinc or MLIR fragment with holes; document expected matches, captures and a deliberately nonmatching example.
- [ ] Decide whether the first slice is just record matching sugar or a bounded host template. Define repeated-hole equality, name binding, extra fields, mismatch versus errors and unsupported native semantics for the chosen slice.
- [ ] Identify shared matching versus host syntax/semantic responsibilities and propose a bounded implementation and documentation example; no generic AST interchange or text-based native identity.
- [ ] The brief and a focused .zdev/_notes/ design document record examples, the user decision, unresolved/deferred options, affected reader/API/host documentation, and a bounded proposed implementation split.

## Validation

- Review the worked examples and alternative against current source and host guides; distinguish proposed syntax from runnable examples.
- Record the user decision; no new product tests are expected.
- git diff --check
- zdev check base --format json
