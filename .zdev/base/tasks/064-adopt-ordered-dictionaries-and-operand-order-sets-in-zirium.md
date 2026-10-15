+++
schema_version = 1
id = "base-064"
key = "zirium-ordered-collection-adoption"
area = "base"
status = "open"
complexity = "advanced"
afk = true
blocked_by = ["base-062"]
+++
# Adopt ordered dictionaries and operand-order sets in Zirium

## Outcome

Zirium ordinary, diff and library query results use one insertion-ordered sortable dictionary and insertion-ordered set algebra.

## Context

Read docs/native-programs.md accepted alignment and the completed shared ordered-collections task. In ../zirium/crates/zirium/src/query.rs and query/{program,diff_program,render,report,literal,comprehension}.rs inspect Map/RankedMap, computed key construction, tally and native set candidates; also inspect Python adapters and host dependency manifests. Ordinary/diff set adapters currently rescan canonical document order; map storage/rendering uses lexical or separate ranked paths. Adopt the accepted shared runtime through a real Git pin. Default domain selection/edge order remains host-owned.

## Boundaries

- Zirium integration only; no MLIR semantic changes, new reflection, source-edit behavior, numeric narrowing, new graph feature or package release. Shared algorithm defects are fixed once in Zirql if necessary, with separately attributable commits.

## Done when

- [ ] Map construction, computed objects and histograms preserve first insertion, stable sorting keeps current-order ties and saved/copied/nested dictionaries retain one dictionary kind through reverse/head/tail. Retained MLIR/diff payloads keep owners and side identity.
- [ ] JSON object member order agrees with library iteration for ordinary, nested and diff dictionary results. Update actual serde_json conversion boundaries or direct serializers; sorting never returns entry-record arrays. Preserve supported native counts and JSON numeric values.
- [ ] Union preserves first appearance left then right; intersection/difference deduplicate in left order. Both operands and discarded handles retain established ownership/side validation. Initial document selections and domain navigation keep their natural order.
- [ ] Focused native/typed/CLI/Python controls cover dictionary insertion and stable ties after reverse/rerank, nested/library JSON, empty dictionaries and ordinary/diff operand-order sets. Update changed public docs and existing expectations rather than preserving compatibility-only RankedMap forms.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.
