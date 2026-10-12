+++
schema_version = 1
id = "base-013"
key = "native-program-runtime"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Add the native program runtime prerequisite for host integration

## Outcome

The existing Zirql crate provides a complete reusable native program and expression runtime, immutable bindings and explicit aggregate limits, with a verified 0.1.0-alpha.2 package candidate. Its independently verified source commit and coordinator-published usable registry prerelease supply the prerequisite for base-004 consumer integration.

## Context

Read the base brief first, including its statement/emission/limit ownership and focused native testing rules, then base-004 and the linked comprehension design for native structured-value and accounting boundaries. src/eval.rs already supplies caller-owned Budget and evaluate with retained context; src/lib.rs owns Located and QueryError, and tests/native_pipelines.rs contains borrowed selections and document-owned operation handles. Add the minimal program module beside these existing owners. Program<S, O, E> contains located ordered binding and evaluate/discard statements. Expressions retain typed host stages, binding reads, grouping, ordered set operands, ordered record fields and typed emitters with named PassThrough or Terminal policies. A lowered statement owns the host-selected implicit output decision. ProgramHost<C> supplies Value, Stage, SetOperator, Emitter and Error plus the concrete copy_value, evaluate_stage, combine, construct_record and emit hooks. The controlling public functions are evaluate_program(program, host, input, context: &mut C, bindings: Bindings<H::Value>, budget: &mut Budget) -> Result<Bindings<H::Value>, EvaluationError<H::Error>> and evaluate_expression(expression, host, input, context: &mut C, bindings: &Bindings<H::Value>, budget: &mut Budget) -> Result<H::Value, EvaluationError<H::Error>>, where H: ProgramHost<C>. Host stage hooks receive the same bindings so native map/key/selector payloads can reenter evaluate_expression with relative input, unchanged context and budget. The new ownership, emission and nested/copy accounting contracts make this advanced work; follow its required planning workflow. This child is the complete core increment, not a checkpoint of incomplete base-004. Keep the accepted parent plan available for resumed integration. After independent whole-child verification and the normal zdev source commit, the coordinator must repeat clean locked package checks and publish usable exact version 0.1.0-alpha.2 before either consumer pins or packages it. The unresolved Zincite source/worktree ownership decision does not block this child and grants it no consumer ownership.

## Boundaries

- Keep one Rust library crate independent of Zirium and Zincite. Change only the allocated core source, native controls, README and Cargo metadata/lockfile. No consumer source, planning records, lifecycle, index, commits or source worktrees belong to the implementation worker.
- Preserve evaluate, NativeInput, NativeResult, Limits and Budget::new and their legacy work/per-stream behavior. Existing context wrappers can retain mutable host state; do not add another pipeline evaluation API, retrofit the error hierarchy, add blanket Clone/Send/Sync/static bounds or serialize native values through a JSON/common document graph.
- Hosts keep surface syntax, native identity/order, stage/value applicability and purity validation, reserved-name policy, semantic facts, rendering and effects. Use typed statically registered host payloads; no arbitrary user callbacks, recursive programs, new edit statements or host dependencies. No comprehension expansion, scalar operator suite, keyed-array/data edits or advanced Zirium effects in this child.
- The child supplies core acceptance only. base-004 remains open for both real CLI/library integrations, host buffering and reuse evidence; Zincite base-126 retains its actual remaining obligations. Do not weaken parent acceptances or assign consumer lifecycle records to the child.
- The coordinator owns release publication under existing authority. Implementers must not publish. Preserve the yanked 0.0.0-prerelease reservation and non-yanked 0.1.0-alpha.1. Publish 0.1.0-alpha.2 only from the independently verified child source commit and matching verified package; no uncommitted runtime may become a consumer pin.
- Initial task-owned path allocation: ["src/program.rs","src/eval.rs","src/lib.rs","tests/native_programs.rs","tests/native_pipelines.rs","README.md","Cargo.toml","Cargo.lock"]. Coordination may extend it only after checking retained parent edits and sibling assignments.

## Done when

- [x] Public located Program/Expression and immutable Bindings execute ordered statements directly over typed host-native values. Core-visible undefined/forward references and duplicate or initially reserved binding definitions fail at their query ranges before emission can run; lookup is read-only and insertion belongs to ordered execution. The documented host lowering contract validates names inside opaque payloads, purity and supported result transitions, including empty streams.
- [x] The shared expression runner handles binding restoration, grouping with current input, ordered set operands, native record construction and explicit emitter sequencing. Lowered statements control bind/discard/implicit output, final explicit emission suppresses duplicate implicit output, and PassThrough versus Terminal behavior remains an explicit host choice. Host errors retain their native type and core failures retain locations without rendered interchange values.
- [x] All statements and nested stage/selector calls retain the same mutable host context and active Budget. Native copies use the charged host copy hook, values move into bindings without copying, and final input is moved where possible. Active nesting/call state is restored after success or failure; nested execution never creates or resets allowances. Existing native pipeline APIs remain usable through context wrappers.
- [x] ProgramLimits { work, results, nesting, stream_items } and Budget::for_program add explicit aggregate produced/copied-entry accounting alongside unchanged legacy Limits/Budget::new. Located charge_results is inactive for legacy budgets. Core producers and native example copy/report/emission paths check known sizes and charge before allocation, repeat charges for repeated materialization/copy, and avoid double admission of already charged rows. README defines one native row/scalar, constructor container and map/record entry as produced units, excludes moving a value into a binding, and charges copied string/rendered bytes to work. Legacy work charges and per-stream ceilings remain unchanged.
- [x] One compact borrowed-selection program control and the existing document-handle ownership pattern demonstrate native binding restoration, relative nested input with retained context, ordering through emissions and constructed records, and located aggregate-copy refusal while individual streams remain under their ceiling. Focused scope/nesting failures use public seams and reuse the same controls; existing native pipeline compatibility and threshold controls pass. No permanent internal call counters, broad matrix, corpus campaign or benchmark platform is added.
- [x] README documents the implemented native program contract, host validation/emission/buffering responsibilities and explicit aggregate units without claiming consumer adoption or comprehensions. Cargo.toml and tracked Cargo.lock identify exact version 0.1.0-alpha.2 while preserving the edition, Rust 1.88 minimum, licensing and independent single-crate packaging.
- [x] The complete core candidate passes required formatting, lint, workspace tests including doctests, record/diff checks and verified Cargo packaging, and receives independent whole-child review before normal zdev completion and source commit. The release handoff requires the coordinator to repeat clean locked list/package/publish-dry-run checks from that commit, publish 0.1.0-alpha.2, and verify registry checksum/package content plus exact-version resolution before any base-004 consumer pin or packaging proceeds.

## Validation

- Before refactoring, run and retain the existing native pipeline baseline. Afterward rerun those public compatibility/work/nesting controls together with the compact native program controls; do not add tests for documentation or metadata alone.
- Run git diff --check, zdev check base --format json, cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace including doctests.
- Inspect the exact candidate package with cargo package --list --locked --allow-dirty and verify it with cargo package --locked --allow-dirty only while the actual implementation is uncommitted. Do not use --no-verify. Require independent verification of the whole child and its actual package before the coordinator creates the normal zdev source commit.
- After the verified child source commit, the coordinator repeats cargo package --list --locked, cargo package --locked and cargo publish --dry-run --locked on the exact clean committed source without --allow-dirty or --no-verify. It then performs the authorized 0.1.0-alpha.2 publication and verifies the registry checksum, package contents and resolvable exact version before either consumer adopts it; preserve both earlier versions and their current yank states.

## Result

Added the independently verified native program/expression runtime, immutable bindings, host emission policies and explicit aggregate accounting while preserving legacy pipeline behavior. The usable 0.1.0-alpha.2 prerelease is published from verified source commit `5dde88958b8e0744ca0e0b9814c19e88b614dedd` and supplies the base-004 integration prerequisite.

Validation:

- Independent whole-child review PASS against Wa2b093301118bb27; unchanged checkout and no findings.
- git diff --check; zdev check base; cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings passed.
- cargo test --workspace and cargo +1.88.0 test --workspace --locked passed: 11 pipeline controls, 2 native program controls and 2 doctests.
- Actual locked allow-dirty Cargo package/list checks and byte inspection passed: exact alpha.2, Rust 1.88, edition 2024, MIT OR Apache-2.0, no dependencies and no AGENTS/.zdev package content.
- Clean committed `cargo package --list --locked`, `cargo package --locked` and `cargo publish --dry-run --locked` passed before authorized publication. Registry archive bytes and checksum match the verified package: `a0970321ca342796a7efcaa201f334e397550a77cb2a895d13a01afeb19873ea`. A fresh exact-version registry dependency resolves and compiles the public program API. Alpha.2 and alpha.1 remain non-yanked; the reservation remains yanked. Core publication does not complete base-004 or consumer tasks.
