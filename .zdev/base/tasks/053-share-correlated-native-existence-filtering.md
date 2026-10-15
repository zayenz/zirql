+++
schema_version = 1
id = "base-053"
key = "shared-correlated-existence"
area = "base"
status = "done"
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

- [x] A shared callable correlated existence-filter path evaluates the nested query independently for each incoming native item, preserves input order and duplicates, and retains the original handles when the nested result is nonempty.
- [x] Zirium Rust/Python where_exists lowers to that shared path. Existing non-operation-result refusals, saved bindings, nested input, function argument scopes and errors retain their behavior; nested failures are never treated as false.
- [x] Empty inputs still receive required preflight; native callback work and result admission use the current budget. Keep the existing typed host surface; a new Zirium textual keyword is not required.

## Validation

- Reuse the existing Rust/Python where_exists controls with one successful relation, one empty relation and a propagated nested error. Demonstrate the shared path with a small native host and a budget refusal.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.

## Result

Shared correlated native existence filtering retains original handles, order and duplicates through a small callable operation. Zirium Rust/Python where_exists delegates iteration while preserving its existing preflight, scopes, native runner and located failures.

Validation:

- Independent whole1/correction0 PASS. Full W3ad7a635c4f5d8f0 comparisons and both repository index/source guards match; no retained validation writes.
- Fresh canonical stable and Rust 1.88 strict Clippy/tests pass in framework and consumer. Formatting, doctests, warning-denied rustdoc and inline/file CLI controls pass; actual locked Git 2220929 and all 19 core source/manifest files match.
- Private CPython 3.13 query/diff suites pass 27 tests; Ruff formatting/lint and ty pass. Source extension identity and seven canonical Python artifact hashes match. Focused native borrowed-handle, empty relation/preflight, nested failure and result-budget controls pass; no guide changes.
