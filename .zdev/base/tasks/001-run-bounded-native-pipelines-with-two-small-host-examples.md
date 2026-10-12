+++
schema_version = 1
id = "base-001"
key = "native-query-core"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Run bounded native pipelines with two small host examples

## Outcome

A reusable Rust crate evaluates a small located pipeline with borrowed source selections and document-owned handles through the same core.

## Context

Read the base brief's executable representation, native values and compatibility sections. The repository currently has planning records only. Start from Zincite language.rs/lib.rs and Zirium query/model.rs, parser.rs and builder.rs as design references. Build one small embedded source host and one operation-handle host in examples or focused public API tests. Their filter/head/count controls establish concrete Rust ownership and extension interfaces before consumer migration.

## Boundaries

- Start with one library crate; examples are not production host adapters or a claim of completed migration.
- Core dependencies must not include Zirium or Zincite. Preserve typed host payloads, native result ownership and byte locations; no JSON handle conversion or common graph model.
- A static extension API is sufficient; exclude dynamic plugins, parser generation and optimization.

## Done when

- [x] The crate builds and exposes located query parsing/lowering plus callable evaluation for filter, Boolean predicates, head and count with host-specific predicate leaves.
- [x] The two embedded hosts use the same pipeline implementation and demonstrate respectively source-borrowed values and document-owned handle identity.
- [x] The public contract defines nested-input/budget ownership, permitted stage result kinds and host extension argument parsing without fixing host navigation vocabulary.
- [x] Unknown-preserving negation, short-circuit evaluation and an exceeded work/item/nesting allowance return the documented result or located error.
- [x] README examples compile and accurately state the implemented subset; Cargo.lock is tracked.

## Validation

- Use one compact native control per host ownership pattern and focused Unknown/limit refusals; avoid tests that mirror enum dispatch or count internal function calls.
- Run the Rust workspace checks and record checks from the brief.

## Result

Built the std-only native filter/head/count core with located parsing and typed lowering, retained host contexts, Unknown-aware predicates and shared bounded execution.

Validation:

- Independent verifier passed six focused integration tests and one compiled README doctest; cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; git diff --check; zdev check base --format json. Snapshot Wd19ef6e4b2c2b10f compared equal before completion.
