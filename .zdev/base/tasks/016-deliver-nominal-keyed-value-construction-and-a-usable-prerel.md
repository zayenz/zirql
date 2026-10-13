+++
schema_version = 1
id = "base-016"
key = "nominal-keyed-core-release"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Deliver nominal keyed value construction and a usable prerelease

## Outcome

Zirql supplies complete callable, bounded nominal-member and explicit keyed/multidimensional value construction through its existing evaluator, with a clean independently verified source/package and a usable registry prerelease required by base-007 packaging.

## Context

Read the base brief, background/comprehensions.md, base-007 and Zincite's keyed/set/empty-axis/numeric/report contract. Alpha.4 already supplies the active Budget and five callable value APIs, but its ConstructedValue, ValueType and SetElementType have no nominal/keyed support. Extend src/value.rs and its public exports directly. Preserve validate_value_expression, free_value_names, constructed_value_type, evaluate_value and copy_constructed_value signatures, existing Comprehension { constructor, clauses, head }, Array/ArrayType and positional callers. Add typed nominal type/member IDs, members carrying IDs/declaration order/retained names, integer/member ValueKey, typed KeyAxis, KeyedEntry, Member and distinct KeyedArray value/type variants, and member-set type identity. Host IDs belong to one immutable host context per request; the core neither establishes membership from spelling/ordinal nor loads models. Add located keyed literal and keyed comprehension variants; reuse Analysis, run and the existing generator clause chain. Zincite's live Program/literal frontend uses the five existing APIs; Zirium's positional holders and exhaustive render/size matches remain pinned to alpha.4 and are outside this child. Follow completed base-015's full source/release acceptance: root owns independent verification, clean source/release commits, approved publication and final lifecycle/records. Independent source/API/package acceptance precedes clean source commit and publication; actual registry archive/checksum/API proof precedes child completion and final record commit.

## Boundaries

- Core-only value/expression APIs, focused public controls, directly affected README/API documentation and necessary package/version changes in the existing single library crate. No consumer changes, parser/CST dependency, generic value-host trait, callback registry, second evaluator, model execution, data writes, semantic-completeness repair or speculative value variants.
- Preserve scalar/positional/nested-array behavior, native pipeline/program APIs, QueryError/Location, numeric/recursive equality, valid empty-set behavior and existing stream ceilings. Nominal identity is host supplied; member arithmetic/coercion and string sets remain refused.
- Generic keyed construction owns typed key arity/identity, duplicate full keys, membership in established axes and row-major order. Sparse/noncontiguous keyed values remain usable. MiniZinc contiguous/rectangular coverage, model/member resolution, source diagnostics and empty-target policy remain Zincite-owned.
- Base-007 remains open for every approved Zincite outcome, including CST lowering, retained public ModelFacts/source identities, enum_member helper resolution, source projections, shape/empty-axis checks, tagged rendering/completeness, aggregate budgeting, dependency/registry packaging and library/CLI/install acceptance. Preserve the peer's canonical/index/record/integration ownership; base-127 remains attended and untouched.
- Publish a fresh usable prerelease only under existing approval after independent acceptance of the complete core and clean locked package. Preserve the yanked reservation and earlier usable versions. Do not complete this child on a source-only pass or publish an incomplete parent checkpoint.
- Initial task-owned path allocation: ["src/value.rs","src/lib.rs","tests/value_expressions.rs","Cargo.toml","Cargo.lock","README.md"]. Coordination may extend it only after checking retained parent edits and sibling assignments.

## Done when

- [ ] Public native member/key/axis/entry/value/type data retains nominal IDs, declaration order, retained rendering names, typed axes/rank and possible element types. All five preserved APIs accept the new forms without a consumer adapter, renderer or CLI; validation and located free-name discovery cover key/head/axis-source expressions and every branch, type discovery checks externally supplied values, and bounded copying retains every identity/axis/key/name.
- [ ] Nominal equality and set deduplication use enum/member IDs, while member sets use declaration order. Externally supplied axes/sets reject inconsistent identity/order metadata. Existing numeric and recursive equality and empty-set semantics remain compatible; incompatible enums, member arithmetic/coercion and string sets receive located refusals.
- [ ] Located keyed literals and comprehensions run through the existing evaluator and generator clause chain. Scalar keys establish rank one; nonempty tuple keys establish rank. Construction checks nonzero rank, compatible key types/identities, duplicate full keys, axis membership and established order. Inferred axes use numeric/member declaration order; explicit axes preserve supplied order. Entries and generator values follow axis positions with the rightmost dimension fastest, including generic sparse/noncontiguous data. Filters run before key/head evaluation without evading candidate charges.
- [ ] Existing ArrayAccess supports integer/member scalar keys for rank-one keyed data and matching tuple keys for multidimensional data; nested accesses remain nested. IndexSet returns the established typed integer/member keys of rank-one keyed data and refuses multidimensional use. Empty native keyed data retains every supplied typed axis. Empty keyed comprehensions use optional explicit axis-source fallback only for zero entries, evaluated outside generator bindings under the same budget, including sources skipped by an empty product; missing rank/identity/axes receive located refusal.
- [ ] One retained Budget bounds analysis, actual-type validation, key/member comparisons, generator candidates before filters, axis/entry/key/type metadata, nested construction and every copied name/value/byte. Check sizes, products, indices and capacities before allocation; repeated evaluation/copying spends again. Errors return no partial value and preserve the existing nesting restoration and native stream limits.
- [ ] One compact public integer/nominal keyed control starts in deliberately different input/output order and exercises all five APIs, identity versus spelling, member-set order/deduplication, rank/axes/row-major data, access/index_set, sparse data, copying and known empty axes. Add only necessary located duplicate/incompatible/unestablished-axis and allowance refusals; reuse scalar/scope/short-circuit/pre-filter and native controls. Required Rust checks, doctests, record checks and locked package validation pass. README/API/package metadata describe the callable subset, budget units, immutable host-ID context and host-policy boundary while preserving edition, minimum Rust version and license.
- [ ] An independent verifier accepts the complete source/API/candidate package before root creates the clean source commit and publishes the approved usable prerelease. Root verifies the clean locked publishable archive and publication dry run, then proves the actual registry archive/checksum against that reviewed package and runs an exact-version temporary registry consumer outside all three repositories using nominal/keyed construction and all five APIs. Only after this full release proof does root complete the child and commit its final records; the recorded verified source revision and available exact version allow the full parent to resume.

## Validation

- Run the compact public integer/nominal keyed control and only the named located refusals with the same active Budget; reuse existing positional/numeric/scope/short-circuit/pre-filter/native controls. No syntax matrix, internal call-count assertions, timing tests or new test framework.
- Run git diff --check; cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace including doctests; cargo package --locked. Preserve Rust 1.88 compatibility. Root runs zdev check base --format json for changed records and obtains independent complete source/API/package acceptance before clean source commit/publication.
- From the clean independently verified source commit, inspect normalized locked package contents and run publication dry run before approved publication. Prove actual registry archive/checksum equality and exact-version nominal/keyed/all-five-API execution using transient files outside the repositories. Record complete release evidence before child completion/final record commit and before parent packaging.
- Confirm canonical Zincite and Zirium HEAD/index/source/record/staged/unstaged/untracked states remain equal to their retained baselines throughout this core-only child; consumer integration belongs to resumed base-007.
