+++
schema_version = 1
id = "base-015"
key = "scalar-positional-core-release"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Deliver the bounded positional value evaluator and usable prerelease

## Outcome

Zirql supplies one independently usable, callable bounded evaluator for checked scalar expressions and positional comprehensions, with a clean verified source revision and a fresh usable registry prerelease for Zincite packaging.

## Context

This is the complete core prerequisite already required by base-005. Read the base brief, background/comprehensions.md and Zincite docs/query-comprehension-contract.md in full. Alpha.3 supplies native program sequencing and an active Budget but no constructed scalar, access, range or comprehension evaluator. Keep the core independent of both consumers. Add located ValueExpression nodes, ValueType information and ConstructedValue data for signed i64, finite f64, fixed Boolean, string, positional array, tuple, ordered record and homogeneous Boolean/integer/float set. Normalized comprehensions retain their positional-array or supported-set constructor, ordered In or single-value Bind clauses, each clause filter and located head. Reuse QueryError/Location and the existing Budget; src/value.rs owns scope, checked operators, access, ranges and iteration. Direct typed validation and located free-name discovery reuse one scoped expression walk for host preflight and demanded projections; evaluate_value receives &ValueExpression, &Bindings<ConstructedValue> and &mut Budget and returns Result<ConstructedValue, QueryError>. Reassess the planned nesting-only validation signature before implementation: validation, located free-global-name discovery and actual-binding type discovery must receive and charge the active Budget, including skipped branches and recursive collection visits. State the revised concrete signatures and call sequence in the implementation handoff before callers rely on them. Provide one bounded constructed-value copy function for host binding and emission snapshots. Evaluation validates every branch against the referenced actual binding types before runtime choice; an unestablished access or element type is only validation information, never a runtime value. The parent retains Zincite CST lowering, source projections, native-versus-constructed routing, tagged rendering, semantic metadata, CLI buffering, dependency pins and install acceptance. The coordinator owns independent verification, clean source commits, approved publication and final task records, following the complete source/release acceptance used by base-014.

## Boundaries

- Complete core-only scalar/positional evaluator, compact public-behavior controls, directly affected API documentation and necessary version/package changes. No consumer edits, parser or CST dependency, model execution, data writing, keyed or enum collections, null, map, manufactured Unknown scalar, generic plugin/value host trait, or separate program runner.
- Preserve native Expression/Program behavior, existing QueryError/Location contracts, legacy host accounting and stream_items limits. Introduce only interfaces needed by the accepted base-005 integration; use the existing single library crate.
- Use one active request Budget for validation, type/free-name walks, expression visits, generator candidates before filters, known expansion/allocation, copied labels/strings/values and nested construction. Check cardinalities, grouped-binder products, lengths and bytes with overflow-safe arithmetic before allocation. Keep necessary core nesting access small; restore active nesting on both success and failure without refunding spent work.
- Publish only a fresh usable prerelease after independent verification of the complete core and clean locked package under existing approval. Preserve the yanked name reservation and all earlier usable versions. Leave base-005 open for the consumer integration; do not checkpoint incomplete parent implementation.
- Initial task-owned path allocation: ["src/value.rs","src/lib.rs","src/eval.rs","tests/value_expressions.rs","Cargo.toml","Cargo.lock","README.md"]. Coordination may extend it only after checking retained parent edits and sibling assignments.

## Done when

- [x] Public typed entry points validate, discover located free global names, evaluate and copy constructed values without a consumer adapter, renderer or CLI. Validation/type discovery/free-name collection and copies are bounded by the supplied active Budget; evaluate_value performs complete actual-type branch validation before evaluation.
- [x] The normalized evaluator preserves immutable scope, located undefined/duplicate/shadowing/forward-reference refusals, discard binders, ordered and dependent clauses, grouped multiple binders whose source runs once before their own names exist and whose last binder varies fastest, single-value generators and fixed-Boolean filters. Positional arrays preserve generation order; records and tuples preserve declared order; supported homogeneous sets deduplicate and use type order, including valid empty arrays/sets.
- [x] Scope, field/tuple/positional-array access, index_set, finite inclusive integer ranges, conditional choice, short-circuit Boolean operations, typed recursive equality/membership, checked sign/arithmetic/comparison and string/positional-array concatenation run in Zirql. Integer overflow, zero division, i64 minimum/-1 div/mod, inexact integer-to-binary64 conversion, nonfinite floats/results and nonzero floating arithmetic rounded to zero fail at responsible locations; div/mod truncate toward zero with dividend-sign remainder. Every unsupported or known-incompatible branch fails prevalidation, while skipped runtime arithmetic and missing access remain unevaluated.
- [x] Expression and value visits, validation, candidates before their filters, repeated expansion, nested construction and copied entries/bytes spend the same retained work/results/nesting allowances. Known sizes are admitted before allocation, false filters cannot evade candidate charges, moving admitted values does not charge them twice, and failures return no partial constructed result. Existing native pipeline/program behavior and budget controls remain compatible.
- [x] The brief's compact typed-core positional report demonstrates out-of-order labels, ordered/dependent/grouped/single-value generators, filtering and record/tuple/array access. Only the named scope refusal, skipped runtime versus static refusal, representative checked arithmetic/exact-promotion/underflow refusals and low-allowance pre-filter refusal controls are added through public APIs. Existing native controls and required formatting, Clippy, workspace tests including doctests and locked package checks pass.
- [x] README and public API documentation explain the callable positional subset, concrete validation/free-name/evaluation/copy sequence, budget units, checked numeric refusals and host rendering/projection boundary without claiming consumer adoption or later keyed/data support. Package metadata exposes the new API in a fresh prerelease while preserving edition, minimum Rust version and license.
- [x] An independent verifier accepts the complete core and candidate package before the coordinator creates the clean source commit and publishes the approved usable prerelease. Registry archive/checksum verification and an exact-version temporary registry consumer outside either repository compile and use the public scalar/comprehension API. The recorded verified source revision and exact available version let base-005 resume its normal Git, registry fallback and cargo-install acceptance.

## Validation

- Run the compact typed-core report and named scope, skipped-runtime-versus-static-error, checked arithmetic/exact-promotion/underflow and pre-filter allowance controls through the public evaluator and active Budget. Reuse existing native pipeline/program controls; no Cartesian syntax/operator matrix, timing assertions, internal call counters or additional test framework.
- Run git diff --check; zdev check base --format json; cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; cargo package --locked. Independent verification checks the complete source/API/package before clean source commit and approved publication; inspect and retain validation writes.
- From the clean independently verified source commit, check the locked package and publication dry run, publish the fresh approved prerelease, verify registry archive/checksum and exact-version scalar/comprehension API resolution using transient files outside the repositories. The coordinator records complete release evidence and commits final task records before the parent resumes consumer work.

## Result

Delivered the standalone bounded scalar/positional evaluator and five active-Budget APIs, preserving native query behavior. Independently verified complete source be8bd8b07d5e7e4fb148f8f2e032ffe0f5e226b9 is published as usable 0.1.0-alpha.4; official registry bytes match the clean package and exact-version scalar/comprehension/API execution passes. Base-005 remains open for Zincite integration; both consumers are unchanged.

Validation:

- Independent corrected source/package PASS, followed by whole-child PASS at We9b076d63271f7c1 with matching coordinator comparison and no validation writes.
- git diff --check; zdev check base --format json; cargo fmt --all -- --check; warning-free cargo clippy --workspace --all-targets; cargo test --workspace (19 public controls and 3 doctests); Rust 1.88 locked check; clean cargo package --locked all pass.
- Clean package/publication dry run and approved alpha.4 upload pass. Official nonyanked registry checksum 879c029bfaaa85736ee580a48c850e710409c0c7f3294abbb2198b09c65580a8 and archive match clean verified source package; exact locked registry consumer exercises scalar/comprehension and all five public functions. Reservation and earlier usable versions are preserved.
- Both canonical consumer HEAD/index/status/diff/untracked and recorded source/record hashes remain equal to retained baseline; accepted parent plan bytes are unchanged.
