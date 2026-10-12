+++
schema_version = 1
id = "base-012"
key = "zirium-work-and-prerelease"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "high"
blocked_by = []
+++
# Support Zirium work charges and prepare Zirql 0.1.0-alpha.1

## Outcome

Zirql exposes the two host work hooks needed to preserve Zirium limits and builds a publishable 0.1.0-alpha.1 package.

## Context

This is the core prerequisite for base-003. Read the base brief and the existing Host contract in src/eval.rs. Zirium charges input length, with a minimum of one, before each stage and adds no row charge for unique or tally. The core currently charges one per stage and one per unique/tally row. Add defaulted Host::stage_work(items, context) -> usize and Host::collection_item_work() -> usize hooks, both returning one by default. Charge stage_work before prepare_stage and collection_item_work before each unique/tally key hook. Zirium can then use input-length.max(1) stage costs and zero additional collection-row work while existing hosts retain their charges. The existing prepare_filter_item hook already supports its filter preparation. Document the cost contract in README.md. Prepare version 0.1.0-alpha.1 so the subsequent consumer Git dependency can package against the same registry version. The coordinator owns independent verification, the child commit and push, and publication from that exact clean verified source before resuming base-003.

## Boundaries

- Change only the two defaulted work hooks, their evaluator charges and contract documentation, one compact public allowance control, and the prerelease package metadata and lockfile.
- Preserve existing default host behavior, native results, error types, located failures, filter preparation, predicate semantics and nesting limits. Keep the core independent of Zirium and Zincite.
- Set exactly version 0.1.0-alpha.1 with Rust 1.88 compatibility, repository and readme metadata, and explicit package inclusion of src/**, tests/**, Cargo.toml, Cargo.lock, README.md, LICENSE-MIT and LICENSE-APACHE. Exclude coordination records from the package and keep source metadata identical to the subsequent Git dependency.
- Use the brief's existing test level. Add one compact acceptance/refusal control for weighted stages and zero additional unique/tally row work; reuse existing controls for default charges, unknown negation and short-circuit behavior. Add no release automation or error-type retrofit.
- The implementation worker must not publish or perform remote actions. The coordinator pushes and publishes the unchanged verified child commit after normal task completion and the clean package checks. Preserve the yanked 0.0.0-prerelease reservation.
- Initial task-owned path allocation: ["src/eval.rs","tests/native_pipelines.rs","Cargo.toml","Cargo.lock","README.md"]. Coordination may extend it only after checking retained parent edits and sibling assignments.

## Done when

- [x] Host::stage_work(items, context) and Host::collection_item_work() default to one, and the evaluator charges them before stage preparation and unique/tally row processing respectively.
- [x] Existing hosts retain their behavior and one compact public control demonstrates accepted weighted stages and zero additional unique/tally row work, including a located allowance refusal.
- [x] Cargo.toml and Cargo.lock use exactly 0.1.0-alpha.1; the manifest declares Rust 1.88, repository and readme metadata, and the required explicit package contents. README.md explains the host work contract.
- [x] Required core checks, Rust 1.88 tests, package content inspection and candidate package verification pass. The coordinator independently verifies the complete child delta before committing it.

## Validation

- Run git diff --check and zdev check base --format json.
- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace.
- Run cargo +1.88 test --workspace.
- Reuse the native controls for default charges and add the single compact public allowance control described above. Preserve the existing unknown-negation and short-circuit controls.
- Inspect cargo +stable package --list --locked --allow-dirty for the uncommitted candidate. Confirm the package contains sources, tests, Cargo.toml, Cargo.lock, README.md and both licenses, and omits .zdev and AGENTS.md. The coordinator repeats cargo +stable package --list --locked after the verified commit.
- Run cargo +stable package --locked --allow-dirty solely to verify the uncommitted candidate. After independent verification and the child commit, the coordinator runs cargo +stable package --locked and cargo +stable publish --dry-run --locked on the exact clean commit before the authorized push and publication. Confirm 0.1.0-alpha.1 is resolvable and non-yanked before resuming base-003, and retain the yanked reservation.

## Result

Added default-one Host work-cost hooks and publishable 0.1.0-alpha.1 metadata. Independently verified the complete candidate against W4d35d7ccc7ce2c19 with no findings; existing default hosts retain their behavior.

Committed and pushed as `7ee35cad2f8d92cbccf29977ff4496987cf7e19d`. Published `0.1.0-alpha.1` from that clean verified commit on 2026-10-12. The registry version is non-yanked and its SHA-256 checksum is `6b96055996a1e0a67fe4ba05a2bdb066e4a719102dbe7cd0f996e9de5dd307d2`, matching the packaged artifact. The original name-reservation version remains yanked.

Validation:

- PASS: git diff --check and zdev check base --format json.
- PASS: cargo fmt --all -- --check, clippy --workspace --all-targets -D warnings, and workspace tests on stable and Rust 1.88 (11 native controls and one README doctest each).
- PASS: stable locked candidate package list and full package verification with --allow-dirty; 11 files include required sources/tests/manifests/lock/README/licenses and omit .zdev/AGENTS. Independent verifier pass; unchanged snapshot comparison.
- PASS: clean stable locked package listing, package verification and publication dry-run at the verified commit. Packaged source files match the checkout, and Cargo package metadata identifies that clean commit. Publication completed and the registry checksum matches the uploaded package.
