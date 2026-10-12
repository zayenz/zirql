+++
schema_version = 1
id = "base-011"
key = "zincite-native-pipeline-core-contract"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-001"]
+++
# Expose the native pipeline contract needed by Zincite

## Outcome

Zirql exposes the concrete typed pipeline, collection, context, error and nesting contracts needed to migrate Zincite read-only queries without changing their native behavior.

## Context

The completed base-001 library currently sequences only Filter, Head and terminal Count over native items, with immutable host context and QueryError-only host failures. The approved base-002 migration needs typed navigation/projection stages, stable unique and string tally, retained mutable stream-kind/semantic context, intact typed host errors and the Zincite written Group/Not nesting boundary. Extend the existing Pipeline/Stage/Host/evaluate design against the actual Zincite query seams in /private/tmp/zincite-base-126/crates/zincite-query/src/{language,inspection,semantic,lib}.rs. Follow the shared brief and accepted base-002 plan. This standalone core prerequisite must be independently verified and committed before the coordinator pushes it and the parent validates Zincite against that exact private Git revision.

## Boundaries

- Use the existing Rust library only and the concrete Zincite host contract. Keep native borrowed items and document handles without Clone, Send, Sync, serialization or consumer-crate dependencies.
- The core owns sequencing, filtering, short-circuit three-valued Boolean composition, head/count and unique/tally algorithms; hosts own navigation/projections, stream-kind applicability even when empty, identity/string keys, domain diagnostics and rendering.
- Preserve the existing core textual frontend terminal-count behavior. Zincite terminal emit/json lowering and all consumer migration, source-byte/output/status compatibility, consumer record reconciliation and consumer commits remain with base-002.
- Do not add programs, bindings, comprehensions, transformations, a callback registry, unrelated policy flags, a new crate or consumer changes. Keep tests at the brief level; coordinator owns records, lifecycle, commits and the already-authorized private push.
- Initial task-owned path allocation: ["src/lib.rs","src/eval.rs","src/parser.rs","README.md","tests/native_pipelines.rs"]. Coordination may extend it only after checking retained parent edits and sibling assignments.

## Done when

- [x] Located typed host stages execute through the existing evaluator and may transform native rows and retained mutable host context; filtering and common stages can enforce host stream-kind applicability on empty as well as populated streams.
- [x] Stable unique retains the first occurrence using host identity keys; string tally returns deterministic sorted keys and counts. These algorithms and applicable head/count share core sequencing, while native context survives stream and scalar/aggregate results.
- [x] Evaluation returns typed host failures intact alongside located core failures, preserving host input/query/model-source distinctions without converting native errors to message strings.
- [x] One explicit lowering/nesting contract supports Zincite Group/Not accounting without charging pipeline, leaf or flat Boolean scaffolding against its legacy allowance; existing core depth behavior and safety caps remain supported, and nested evaluation uses the active budget.
- [x] Host scans and admitted native rows can use that same active Budget without double charging existing admission work. Public native controls demonstrate the concrete host-stage, common-operation, retained-context, typed-error and logical-nesting integration contract, and required checks pass.

## Validation

- Establish the existing native pipeline controls before refactoring and reuse them after the change. Add only compact public controls for the actual missing contract, including stable ordering/tally, empty-stream applicability, intact host errors and the logical nesting boundary; avoid internal call-count tests or generic matrices.
- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace.
- Run git diff --check and zdev check base --format json. Obtain normal independent verification before the coordinator commits and pushes the attributable core change; report the public contract for the resumed base-002 integration.

## Result

Expose typed native host stages, retained mutable context and errors, stable unique/tally, admitted-row input and Group/Not nesting through the existing shared evaluator.

Validation:

- Independent verifier pass at W260111fb76f8736f; no findings, no validation writes, exact four-file source delta.
- cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace: ten native controls and README doctest pass.
- git diff --check and zdev check base --format json pass; coordinator snapshot comparison equal before completion.
