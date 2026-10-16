+++
schema_version = 1
id = "base-087"
key = "integrate-general-filter-consumers"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-086", "base-082", "base-083"]
+++
# Unify predicate expressions across Zirium and Zincite release candidates

## Outcome

Both consumers use the shared named and unnamed predicate language consistently at their existing predicate positions, with refreshed reviewed-core pins and release evidence.

## Context

Consume the preceding task's reviewed Zirql0.1.0 Git revision and shared predicate contract. Read the brief's predicate inventory/subject/scope rules. Zirium query/parser.rs has separate native predicate parsing and constructed-filter routing; comprehension.rs constructed_filter_call, report.rs walkers/validation and program/functions routing determine value/native/function contexts. Native filter/root, paths targets and native match where must use the shared expression contract with existing operation subjects and domain predicates. Zincite program/composition.rs currently calls hostless read_value_suffix and separately parses native predicate leaves; function/value walkers in functions.rs and matching/path callbacks retain validation/evaluation scope. Also inspect Zirium query/diff_text.rs and diff_program.rs filter/root, and Zincite public Query/legacy Program parsing plus language.rs FilterElements and structured.rs scalar comparison/edit masks. General scalar predicates must obey the existing Data operator/effect policy, preserve direct scalar/member identity and keep keyed/matrix/comment/directive/atomic-write validation. Adapt all these boundaries without reimplementing a predicate grammar/evaluator. Accepted packaging candidates082/083 pin6b6184f; refreshed candidates must replace that pre-filter source and supersede its artifacts.

## Boundaries

- Allow shared named/unnamed expression predicates alongside host domain leaves at every inventoried predicate position. Keep existing MLIR/MiniZinc subject meaning, projections/facts, native identity/ownership, effects, edits, completeness and whole-request buffering. No independent predicate evaluator, arbitrary new native fields, first-class functions, selector/relation overloads or support-matrix expansion.
- Use isolated sources and separately attributable serial commits. Root owns Git transport/pins/locks, zdev records and candidate bookkeeping. No publication/tag push/release assets/live site import.

## Done when

- [ ] Both consumers accept p => predicate and implicit _ expressions using shared scope/type/purity/call/budget rules at every brief-inventoried position: constructed/native filter, native root, paths targets, match guards, Zirium diff filter/root and Zincite filter_elements, with their existing subjects and host expression/effect policies. Field comparisons, Boolean composition, lexical captures, pure calls and existing domain predicates compose naturally. Saved/nested/declared-function contexts where supported use the same rules.
- [ ] A few public library/CLI programs across both hosts and CSV demonstrate named/unnamed filter-comprehension equivalence, stable order, nested _ rebinding with named outer capture, typed-empty validation and located predicate failures. Representative non-filter positions are exercised without a generic matrix. A later predicate failure after earlier output leaves stdout empty.
- [ ] Manifests/locks pin the actual reviewed core and keep registry0.1.0 contracts. Reader guides/reference/release notes explain the common predicate forms and host subject/operation limits with runnable examples. Old draft artifacts are superseded; final artifact acceptance remains084.
- [ ] Required affected host Rust/MSRV/Python and site/link/desktop/mobile checks pass, reusing unchanged packaging proof where applicable. Root independently verifies and records separate commits and refreshed candidates.

## Validation

- Use a few focused existing-style public controls for named/unnamed constructed and native predicates, representative path target/match guard, native ancestor root, diff and scalar-edit predicate controls, nested scopes and later-error buffering. These are distinct execution boundaries rather than a form-by-position matrix. Inventory all call sites against the shared contract; do not expand into a syntax matrix. Replay existing domain-leaf predicate controls.
- Run both affected hosts' required Rust checks, Zirium's existing 1.88 and Python rebuild/identity/suite/lint/type checks, warning-fatal docs and affected site builds/links/UI. No new compiler/platform matrix.
- Inspect current locked dependency identities and normalized release requirements. Refresh local install/artifact proof only where the source change invalidates prior evidence; base-084 checks final seven-package graph, exact artifacts and hosted jobs before publication handoff.
