+++
schema_version = 1
id = "base-077"
key = "projection-values-in-prepared-programs"
area = "base"
status = "open"
complexity = "advanced"
afk = true
blocked_by = ["base-076"]
+++
# Execute native projections inside prepared values and functions

## Outcome

Existing native projections compose with shared scalar expressions and named function results through the active Zirql runner, independent of incidental frontend routing.

## Context

A concrete existing Zirium composition gap remains outside base-076. On the one-operation input "test.a"() : () -> (), fn n()->int{1} project(row,{"z":one(row | names)}) fails with written function entry was not prepared in this scope, while the same projection without the declaration and the names-array field with the declaration succeed. fn make(root:Operation)->sequence<record {"z":int}> {root | project(row,{"z":1})} make(input | head(1)) also fails. Root independently reproduced both failures against rebuilt baseline and base-076 binaries. The frontend constructed-start heuristic can route the complete projection as a shared value; projections in named function bodies use that route inherently. Session prepares direct Projection field entries, but ReportHost does not declare nested Projection fields to scoped native argument discovery or execute them with the active argument runner. Read Zirium parser/comprehension starts, functions.rs Session::prepare_expression, report.rs scoped_value_arguments/evaluate_stage_with_arguments, and Zirql NativeArguments plus the shared projection machinery delivered by base-076. A parser-precedence-only repair is insufficient.

## Boundaries

- Restore composition of existing supported projections and scalar/function expressions. Reuse the active prepared-function/native-argument machinery and shared projection type/row execution; no second evaluator, host-owned subject/field loop, generic plugin protocol, parser rewrite or new projection representation.
- Preserve all-written admission before subject enumeration, lexical outer/subject scope and no-shadowing, original versus singleton-relative input, written field/subject order and duplicates, native ownership/source coordinates, located active function failures, finite cumulative budgets and later-error output buffering. No reparsing or function-body preparation per subject.
- Keep native selection, type facts and explicit scalar/rendering conversion with the host. Change only the affected framework/consumer integration and directly affected documentation; no symmetry consumer pin, package publication, deployment or broader edit policy.

## Done when

- [ ] The minimal one(row | names) projection with an unrelated function declaration succeeds and returns the same intended scalar report as its plain counterpart through actual library and inline/file CLI paths.
- [ ] A named function can return a sequence of projected records, and a projection nested in a value field uses the same shared execution path. Prepared fields execute through the active native argument runner instead of missing Session entries; directly prepared projections remain correct.
- [ ] Compact public controls establish valid typed empty reports, all-written invalid-field refusal on empty input, outer/subject and relative/original scope, duplicate subjects and field order, located active function failure and later-error empty stdout. Reuse the meaningful base-076 owner/budget controls rather than adding a generic matrix.
- [ ] If a shared API changes, the actual affected consumer manifest, lock and locked metadata resolve the reviewed pushed Git revision. Required changed-repository checks pass, and any affected API guide describes current behavior and caller responsibilities.

## Validation

- Run focused existing projection/function library controls and rebuilt actual CLI inline/file programs for the concrete reproductions, including empty and later-error cases. Inspect that shared row orchestration and one cumulative request are retained. No internal callback-count, timing or implementation-mirroring tests.
- Run changed-repository diff/format, default and explicit Rust1.99 warning-fatal Clippy and locked workspace tests; shared Rust1.88 tests and required Zirium stable/1.88 all-target/all-feature tests/Clippy, doctests and warning-clean docs. Rebuild the current local Python extension, prove import identity and run the full compatibility suite; unchanged Python lint/type evidence may be reused and configured CI covers its interpreter matrix.
- Run zdev checks for records; existing site build/link and desktop/mobile review only for directly affected guide pages. Root independently verifies the whole task, integrates serially and commits framework/consumer changes separately, preserving unrelated work and pinned Git development.
