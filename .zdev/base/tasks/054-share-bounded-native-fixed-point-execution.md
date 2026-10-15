+++
schema_version = 1
id = "base-054"
key = "shared-native-fixed-points"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Share bounded native fixed-point execution

## Outcome

Zirql owns native fixed-point iteration, convergence and cycle detection while Zirium supplies body operations and domain validity.

## Context

Zirium query.rs::evaluate_host_stage Fixpoint and query/diff_program.rs::fixed_point independently own replacement iteration and Brent cycle detection. Zirql’s explicit loops and worklist collections do not supply this existing fixed-point contract. Read query-language.md Closure and fixed points; generic fixpoint differs from domain-specific closure and the optimized fixpoint(closure) worklist.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [ ] Shared callable execution and common fixpoint(query) reading repeatedly replace the current selection with the body result until ordered values, including duplicates, are unchanged. Intermediate selections do not accumulate implicitly; deterministic repeated non-fixed states fail.
- [ ] Ordinary and diff Zirium paths delegate generic iteration/cycle handling to Zirql. Preserve native kind restrictions and diff-side preservation, nested fixed points, saved bindings and existing per-iteration emitters including the final unchanged iteration.
- [ ] Iterations, comparisons and retained checkpoint copies use the active work/item/result/nesting limits and report located failures. Retain bounded checkpoint storage rather than storing every visited selection.
- [ ] Domain closure semantics and the specialized fixpoint(closure) worklist remain host-owned and compatible; this task does not replace the optimized traversal or alter edit/emission policy.

## Validation

- Reuse fixed-point controls for unchanged input, convergence to empty, a two-state cycle, growing duplicates stopped by a limit, nested execution and emitter ordering. Include diff-side refusal and a shared-library control using non-domain integer identities.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.
