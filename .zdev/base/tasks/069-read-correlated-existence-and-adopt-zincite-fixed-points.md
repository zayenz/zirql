+++
schema_version = 1
id = "base-069"
key = "common-existence-and-fixedpoint-adoption"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = ["base-067", "base-064"]
+++
# Read correlated existence and adopt Zincite fixed points

## Outcome

Common where_exists text queries and existing fixed-point execution compose with native queries in both hosts.

## Context

Read background/collection-language-contract.md native existence contract and the brief fixed-point contract. Core src/existence.rs already implements filter_native_exists, but src/reader.rs lacks a retained nested-query hook for it. Zirium typed where_exists already works and its shared text adapter can lower the same operation. Core fixed_point.rs and reader already implement replacement iteration and bounded cycle refusal; Zincite composition lacks adoption. Reuse complete-or-error host adjacency/nonemptiness primitives and native identities rather than implementing reachability or a second iteration engine. The accumulating Zincite acceptance example depends on the newly adopted native set algebra.

## Boundaries

- No new graph closure, path/reachability operation, generic operation registry, fact expansion or data-edit policy. Reuse completed core algorithms and host worklist/reference primitives.

## Done when

- [x] The common reader retains and lowers where_exists(query) through the smallest necessary hook; supporting ordinary/diff/typed host paths share existing filter_native_exists behavior rather than a host-only duplicate.
- [x] Nonempty pure singleton-relative relations retain original input subjects/order/duplicates; known complete empty relations reject them. Unknown/unresolved nonemptiness and nested errors refuse at responsible locations; known unsupported selectors/effects preflight on empty input.
- [x] Zincite fixpoint(body) uses existing same-kind replacement iteration, ordered duplicate-sensitive equality, one bounded checkpoint/cycle refusal and the active budget. An unchanged/empty result succeeds; a repeated non-fixed result refuses. Accumulation requires explicit union in the body.
- [x] Preserve native owners/side identity, saved inputs and host limits/effects. Incomplete adjacency cannot prove convergence; use established complete-or-error primitives when needed and retain other report limitations. No implicit emission/edit permission.
- [x] Compact inline/file/library controls demonstrate correlated existence, a known-empty relation, an unresolved/error refusal, one converging accumulating graph and one alternating cycle, plus existing later-error buffering. Update affected host/shared guides and real Git pins with separate commits.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.

## Result

Read correlated native existence through Zirql in both hosts and adopt shared bounded fixed points in Zincite, preserving native context and incomplete-result refusals.

Validation:

- Independent whole review, one bounded correction and fresh focused recheck passed; all required canonical Rust compatibility checks and actual Git dependency resolution passed.
- Typed, text, ordinary, diff and prepared paths, scope/owner/order/completeness/cycle and later-error buffering controls passed. All three sites build, 757 changed-page links and exact guide commands pass; nine guides rendered at desktop and mobile widths.
