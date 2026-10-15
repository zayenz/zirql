+++
schema_version = 1
id = "base-067"
key = "zincite-native-ordering-and-sets"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = ["base-062"]
+++
# Adopt typed native ordering and stable sets in Zincite

## Outcome

Zincite exposes shared native ordering/extrema and operand-order set algebra with its established typed keys and identities.

## Context

Read background/collection-language-contract.md native alignment after shared collection order. In ../zincite/crates/zincite-query/src/program.rs, program/composition.rs, pipeline.rs and program/{functions,worklists}.rs inspect singleton projections and native identity. SetOperator is currently () and native ordering hooks are missing. Adopting the ordered core also changes native tally storage from BTreeMap to ordered pairs and changes constructed Sort selectors; migrate their existing host callers, copies and rendering while retaining dictionary output and first-seen tally order. Broader retained map stages remain the following task. Core src/ordering.rs only projects Count/Text, whereas value.rs already compares homogeneous typed int/finite float/string/one-nominal keys. Extend/reuse that shared comparison with this concrete native consumer; do not add parallel algorithms or preserve obsolete APIs solely for compatibility.

## Boundaries

- No new native identity inferred from text, graph algorithm, comparator registry, common nominal resolver, broader MiniZinc evaluation or edit/emit permission. Core typed comparison is implemented once and reused by subsequent dictionary adoption.

## Done when

- [x] Direct and typed native sort/min/max/min_all/max_all and *_by(selector) share existing algorithms and preserve admitted native kind/owner. Scalar keys are present homogeneous i64, finite f64, exact strings or one coherent nominal type; Count conversion to signed keys is checked.
- [x] Stable sort keeps ties; single extrema keep the first tie; all-ties forms retain every tied item in input order; empty native extrema produce a same-kind empty selection. Missing/absent, Boolean, mixed int/float, incompatible nominal keys and known unsupported empty-input kinds/selectors fail as selected.
- [x] Native union/intersect/except reuse existing core set machinery with current host identities: union first-seen left then right, intersection/difference deduplicated left order. Validate both operands/owners even for discarded/empty results; admit only kinds with established identity.
- [x] Keep selectors pure, original-input/singleton scope explicit, uncertainty/limitations and source coordinates intact under one budget. Existing native count/value domains in Zirium are not narrowed merely to add typed Zincite comparison.
- [x] Compact numeric and nominal fixtures demonstrate stable first/all ties, same-kind empties, typed refusals and native set [C,A,C] versus [B,A] yielding C,A,B / A / C. Reuse host checks and update actual dependency pin and guides.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.

## Result

Zincite exposes shared typed native ordering and extrema, operand-order sets and first-seen tally dictionaries while retaining native owners, coordinates and report limitations.

Validation:

- Independent whole review plus one bounded correction and focused recheck passed; all three accepted findings resolved, frozen context and both repository guards equal.
- Core and Zincite required formatting, Clippy, workspace tests, core Rust 1.88 and current-stable Rust 1.99 checks passed; actual locked Git dependency verified.
- Focused text/typed/inline/file controls passed for typed scalar and nominal keys, stable ties, native empties, mixed node storage, owners/coordinates, pure selectors, set operand order, tally JSON, CLI buffering and atomic edits.
- Both sites check/build and changed-guide links passed; all core 20 pages/966 internal links and Zincite 22 pages/1065 links passed, with six changed guides and corrected paragraph inspected at desktop/mobile widths.
