+++
schema_version = 1
id = "base-066"
key = "common-indexed-array-syntax"
area = "base"
status = "open"
complexity = "advanced"
afk = true
blocked_by = ["base-065"]
+++
# Read indexed arrays with optional declared index sets

## Outcome

Common indexed literals and comprehensions retain inferred or explicitly declared integer/nominal dimensions through shared evaluation.

## Context

Read docs/values.md selected array-shaped syntax and existing keyed construction/runtime guides. src/value.rs KeyedArray currently accepts concrete axes and KeyedComprehension only has zero-cell empty_axis_sources fallback. The selected [key:value] and tuple-key comprehensions plus optional axes(...) sources require an explicit located expression-source path that runs before entries/generators for empty and nonempty results. This differs from the existing lazy fallback, so name and implement the distinction honestly. Reuse existing rank/identity/duplicate/row-major normalization and supplied host nominal values. Consumer seams are Zirium crates/zirium/src/query/parser.rs::shared_value ordinary-query call admission (currently excludes axes) and Zincite program/composition.rs::lower_composed_value -> functions.rs::lower_function_value/CST lowering. Real pin updates and bounded adapter changes are required; dependency presence alone is not acceptance.

## Boundaries

- No new collection value hierarchy, automatic filling/compaction/flattening, string array axes, host fact expansion or data-edit policy change.

## Done when

- [ ] Literal and comprehension indexed-bracket forms infer integer/nominal key dimensions, and optional axes(...) evaluates its sources once left to right in surrounding scope before entries/generators. Explicit domains retain their order, unused keys and typed empties for zero and nonzero cells.
- [ ] Tuple keys preserve rank and row-major shape; supplied keys outside declared domains, duplicates, incompatible nominal identities, wrong rank and unestablished empty dimensions fail with original locations. [] alone stays positional, while [] axes(...) retains declared keyed shape.
- [ ] Existing consumer empty_axis_sources behavior remains a deliberately lazy fallback until migrated; it is not misused as the always-retained declared-domain implementation. No generalized arbitrary-key dictionary or stringification of nominal/integer indices.
- [ ] Located shared validation/name/type/evaluation/copy paths retain the selected behavior. Both tools adopt the delivered shared Git revision: Zirium inline/file/library entry points accept the indexed/axes forms through required dispatch/admission glue; Zincite natural MiniZinc indexed forms lower to the same shared runtime and retain established model/target/edit checks and deliberately lazy empty-axis fallback where that host form uses it. No common model resolver or second evaluator.
- [ ] Use the compact sparse rank-two integer/nominal and typed-empty/unused-domain controls across common callable/text paths and actual Zirium/Zincite inline/file/library paths, choosing the permitted natural host spellings and existing host-supplied nominal context. Add focused scope/type/budget refusals, update actual dependency pins/current guides and run affected host checks; no syntax matrix.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.
