+++
schema_version = 1
id = "base-076"
key = "shared-prepared-projection"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = ["base-066"]
+++
# Use shared projection execution with prepared functions

## Outcome

Zirium projects rows and fields through shared Zirql execution both with and without prepared functions, retaining the existing native and result contracts.

## Context

The brief assigns general query evaluation to Zirql. Zirium crates/zirium/src/query/report.rs::project currently delegates ordinary projections to zirql::evaluate_projection_with_types, but its context.functions branch enumerates subjects, installs bindings, evaluates fields and constructs rows locally. Existing functions_prepare_fields_arguments_and_all_written_bodies_before_execution in tests/query_dsl.rs exercises this branch: fn bump(n:int)->int{n+1} fn label()->string{"example.root"} filter(op(label())) | head(bump(0)) | project(row,{"cost":bump(row | children | count)}) | json returns cost 3 on examples/cli/subtree-cost.mlir. The corresponding plain projection also returns 3. Read shared src/projection.rs::evaluate_projection_with_types, PreparedFunctions/native argument runners and Zirium functions.rs Session preparation/run plus ReportHost subjects/project_value. Scalar, function and loop evaluation already belongs to Zirql; this task removes the remaining function-bearing projection orchestration copy. Reconcile current indexed-array and scalar-result changes before touching these seams.

## Boundaries

- Reuse existing shared projection, prepared-function and native argument machinery. Keep one row/field execution path where practical; no second evaluator, generic plugin system, parser rewrite or new language feature.
- Preserve all-written preflight before subject enumeration, lexical subject binding, original and relative input, field order, subject order and duplicates, native ownership/source coordinates, located failures and cumulative finite budgets. Host adapters keep domain selection, value views and explicit rendering. Do not require exact incidental work counts.
- Preserve current library and CLI output kinds and JSON envelopes, scope/purity/empty-input rules and later-error output buffering. Change another consumer only if its actual compilation or shared contract requires it; no symmetry pin changes, publications, deployment or broader edit policy.

## Done when

- [x] A callable shared path executes projections whose fields use the existing prepared function request, without reparsing or preparing function bodies per subject or starting another request. Zirium uses it and removes its local subject/field iteration, scope and row-construction duplicate.
- [x] The existing function-bearing projection and its plain counterpart return the same intended cost report through actual library and inline/file CLI paths. Native selections and projected native fields retain their owners and original source ranges until the existing explicit rendering boundary.
- [x] Compact existing public controls establish ordered duplicate subjects and fields, surrounding/subject scope, empty input with required all-written validation, a located field failure and later-error empty stdout. Reuse existing preparation/function/budget controls; no callback-count tests, timing assertions or generic matrix.
- [x] Actual Zirium manifest, lock and locked metadata resolve the reviewed shared Git revision. Required core and Zirium checks pass, and any directly affected API guide describes current behavior without process narration. Broader reader-guide editing remains in its own task.

## Validation

- Run changed-repository diff/format checks, default and explicit Rust1.99 Clippy warnings denied and locked workspace tests; shared Rust1.88 tests and required Zirium stable/1.88 all-target/all-feature Clippy/tests, doctests, warning-clean rustdoc and Python query compatibility checks.
- Run the concrete existing projection program and its plain counterpart through rebuilt library/inline/file paths. Extend a compact existing control only where it verifies the named execution boundary; no new benchmark or test framework. Run existing documentation/site checks and desktop/mobile review only for affected reader pages.
- Root retains serial integration, unrelated-change preservation, independent whole verification and separately attributable framework/consumer commits. Use pinned Git development without registry releases.

## Result

Execute directly prepared report projections through shared Zirql row execution, retaining typed empty schemas and located function failures.

Validation:

- Independent whole review 1 and focused recheck 1 pass after the single paragraph-boundary correction; exact native snapshot and all repository guards match.
- Core and Zirium required Rust format, default/1.99/stable/1.88 Clippy/tests, doctests and warning-clean docs pass. Rebuilt canonical Python imports pass with 175 tests and 2 existing skips.
- Actual plain/prepared cost-3 library and inline/file reports, duplicate/order/scope/empty admission and located later-error empty-stdout controls pass. Manifest, lock and metadata resolve reviewed Git d8c7710903ae6402624e715625334f27349a2092.
- API site check/build, 992 links and both guides on desktop/mobile pass; consumer/domain boundaries and unrelated work preserved.
