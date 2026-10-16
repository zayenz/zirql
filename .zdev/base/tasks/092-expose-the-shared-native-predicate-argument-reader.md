+++
schema_version = 1
id = "base-092"
key = "public-native-predicate-reader"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "high"
blocked_by = ["base-091"]
+++
# Expose the shared native predicate argument reader

## Outcome

Hosts can read named, destructured and implicit predicate arguments containing native leaves and shared Boolean expressions through the existing Zirql reader without duplicating Boolean parsing.

## Context

The base-089 read-only planner found that normal root and separate diff filter/root require the mixed native predicate argument reader. src/reader.rs already implements Reader::native_predicate_argument with optional Pattern scope, mandatory explicit-header admission, shared native/value Boolean composition and original source ranges. The public read_predicate_argument returns only a value-body PredicateArgument and cannot preserve the existing mixed native Predicate tree. Export a thin read_native_predicate_argument(reader, host, locals, budget, nesting) -> Result<Predicate<H::Predicate>, QueryError>, initialized like read_predicate_argument and calling the existing private method. Read the brief Scoped predicate expressions and One shared query reader sections, src/lib.rs exports and the existing tests/native_programs.rs host fixtures. Both consumers remain separate integration tasks; base-087 can use the unchanged existing API.

## Boundaries

- Reuse the existing reader, AST, admission, Truth/evaluation and cumulative Budget machinery. Add no grammar, evaluator, host Boolean probe stage, source rewriting, configuration or optional feature.
- Core API/export, a focused existing-fixture public control and concise current-boundary documentation only. No consumer source/lock changes, registry publication, release tags/assets or site deployment. Root owns Git transport, records, dependency pins and serial acceptance.

## Done when

- [x] The public root re-export and reader entry accept the same original cursor, locals, Budget and remaining nesting contract as the existing value-body reader and return the existing mixed Predicate tree. Named/pattern headers, implicit _, native leaves and shared bodies retain their existing scopes and admission.
- [x] A focused public adapter control composes an established native leaf with a named or implicit shared body and demonstrates original located refusal and low-limit host reentry. Reuse existing fixtures; no syntax-by-position matrix.
- [x] Current adapter/API documentation explains which public reader consumes a mixed native predicate versus a value-only body. Required Rust/MSRV, warning-fatal docs, diff and zdev checks pass; affected site checks and Root reader presentation review pass when prose changes.

## Validation

- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace, plus existing Rust 1.88 tests and warning-fatal docs/doctests. Reuse existing fixture controls for runtime Truth/header scope; add only the direct public boundary check needed for this export.
- Inspect the actual Zirium root/diff call sequence to confirm this export removes the missing boundary without requiring another generic parser. Check original cursor/ranges and one cumulative low-limit reentry refusal.
- Run git diff --check and zdev check base --format json. If reader-facing prose changes, run the existing site check/build/local-link checks and Root desktop/mobile review; no package version or distribution matrix change.

## Result

Exposed the existing shared mixed native predicate reader for host arguments, preserving scope, locations and cumulative limits.

Validation:

- Independent whole review passed public adapter behavior, Rust/MSRV tests, fatal Clippy and docs, formatting and zdev checks.
- Site check/build and local links passed; desktop/mobile reader review matches the exact guide and rendered bytes.
