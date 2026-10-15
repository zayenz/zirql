+++
schema_version = 1
id = "base-063"
key = "direct-zincite-expression-routing"
area = "base"
status = "done"
complexity = "standard"
afk = true
blocked_by = []
+++
# Use direct Zincite expressions and shared constructed suffixes

## Outcome

Zincite runs short direct and saved native/constructed programs consistently through the existing shared expression machinery.

## Context

Read the brief Zincite alignment section and docs/native-programs.md accepted alignment. Zincite base-140 already removed source/named-literal aliases and supplies let-only bindings and implicit final output. Existing crates/zincite-query/tests/fixtures/native-composition.zq is a working baseline. In ../zincite/crates/zincite-query/src/program.rs and program/composition.rs inspect native dispatch and MiniZinc CST lowering. Core base-061 already supplies constructed current input and filter(starts_with(...)) | count; adopt it rather than reimplementing. The current query("...") wrapper has no compatibility requirement and should be removed in favor of direct expressions.

## Boundaries

- Consumer changes are Zincite frontend routing and its documentation/controls only. No new syntax family, collection algorithm, model evaluation, formatter feature or effect policy. Read committed host contracts and preserve foreign edits.

## Done when

- [x] A saved native selection supplies names, count and multiple later statements; saved constructed function results support existing sort/unique/tally and shared string filter/count in inline, file and library programs.
- [x] Sibling report fields independently read their intended saved/current input, and an unused function declaration does not change expression routing. Known-invalid stages after saved values remain located even on empty input.
- [x] Direct native expressions replace the obsolete query-string wrapper; update actual tests/examples and remove unused decoded-string adaptation where it has no other concrete use. Preserve original source locations, let-only scope, optional final semicolon and implicit final output.
- [x] Reuse existing functions/patterns/conditional/match/fold/scan/loops, original-input reads and host emitter/effect/atomic-edit rules. Demonstrate a later-error CLI request produces no partial stdout, without adding a second evaluator.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.

## Result

Zincite uses direct shared expressions for saved native and constructed results; obsolete quoted wrappers are removed. Consumer commit f17fe54996f5723223505da84622c522fb0f3fff.

Validation:

- Independent whole-task review passed without corrections; canonical formatting, Clippy with warnings denied and all workspace tests passed.
- Canonical site check/build passed; 22 pages and 1058 internal links have no broken targets; changed guide and pipeline reference inspected on desktop/mobile.
- Actual library and both CLI inline/file controls preserve shared suffix results, original ranges, sibling input, native owners and empty stdout on later failure; runnable guide returns integer 2.
- Native context comparisons and exact consumer index/working-tree guards matched; unrelated edits preserved.
