+++
schema_version = 1
id = "base-053"
key = "shared-correlated-existence"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Share correlated native existence filtering

## Outcome

Zirium’s typed where_exists uses shared correlated evaluation that other Zirql hosts can reuse.

## Context

Zirium query/builder.rs::evaluate_structured implements StructuredStage::WhereExists as a host loop; OpQuery::where_exists is exposed to Rust/Python. Each singleton operation runs a nested operation query and is retained when that query returns a nonempty selection. Zirql already has matching, comprehensions and exists reductions; prefer lowering through those mechanisms if it preserves native results, rather than inventing another evaluator.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [ ] A shared callable correlated existence-filter path evaluates the nested query independently for each incoming native item, preserves input order and duplicates, and retains the original handles when the nested result is nonempty.
- [ ] Zirium Rust/Python where_exists lowers to that shared path. Existing non-operation-result refusals, saved bindings, nested input, function argument scopes and errors retain their behavior; nested failures are never treated as false.
- [ ] Empty inputs still receive required preflight; native callback work and result admission use the current budget. Keep the existing typed host surface; a new Zirium textual keyword is not required.

## Validation

- Reuse the existing Rust/Python where_exists controls with one successful relation, one empty relation and a propagated nested error. Demonstrate the shared path with a small native host and a budget refusal.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.
