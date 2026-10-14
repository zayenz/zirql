+++
schema_version = 1
id = "base-031"
key = "optional-values"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-030"]
+++
# Use optional lookup and lazy defaults in both query languages

## Outcome

Both consumers can report absence, inspect membership and supply defaults through one optional-value evaluator.

## Context

Read the selected bundle’s opt/default/lookup contract and the comprehension contract. Start with src/value.rs and src/reader.rs and both consumers’ structured expression lowering. Current strict accesses fail on missing data and current constructed values have no absence variant. Extend the declared runtime contract from runtime-type-contracts.

## Boundaries

- Integrates Zirql, Zirium and Zincite read-only reporting; does not expand MiniZinc data-edit serialization or solver semantics.

## Done when

- [x] Shared optional values, occurs/absent, lookup/has_key and lazy default implement the selected distinctions, including missing versus present-absent cells and incompatible key/field errors.
- [x] Zincite <> and Zirium null lower to the same semantics in ordinary reports, with shared optional runtime contracts and located compatibility errors. The later function frontend tasks add opt signature syntax; existing external CLI/library parameter declarations and transport remain int/string/bool.
- [x] Present false/zero/empty values keep their meaning; skipped fallbacks do no runtime work, and arithmetic/host/budget failures propagate. Ordinary arithmetic and ordering require presence handling.
- [x] Both report renderers preserve omission versus present absence. Existing strict access is compatible; unsupported optional values in data-edit targets fail before any write.
- [x] Update user guides and record separate consumer commits and pinned Zirql revisions.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse a compact sparse/optional-field report for presence, defaults, type errors and later failure; add only focused controls for these new distinctions.

## Result

Both consumers now use shared optional values, membership, lookup and lazy defaults with retained model schemas, sparse and mixed nested storage, and native identities.

Validation:

- Independent whole-task review and focused rechecks passed with no remaining findings after the explicitly approved additional mixed-array correction; all prior review history was retained.
- Zirql formatting, warning-fatal workspace/all-target Clippy, stable and Rust 1.88.0 locked tests and doctests passed; exact source CI 38549617818 passed at 9c8bef3f1c8e0241dc5781ec5408162b24121b43.
- All 44 affected host Rust/Python checks, 20 public proof replays and four focused controls passed independently; each fresh CPython 3.11-3.14 environment passed 171 tests with two existing skips.
- Inline, file, Rust/Python builder and native reports agree; later errors leave stdout empty. Wrong fields/elements/keys, missing strict access and unsupported optional data edits refuse while preserving original files and modes. Nonoptional MiniZinc instance-only candidate acceptance passed.
- Documentation checks/builds and actual desktop/mobile navigation, search and keyboard review passed. All 237 rendered outputs and unchanged site inputs were verified, retaining the focused sparse-guidance correction.
- Zincite c7a51636f16c37b8c93ae6ac5ca9c32b7bce7388 and Zirium 077a56f9c143a407c39a75b511c0dabec132aa01 were committed separately and pushed; both real manifests and Cargo.lock resolve Zirql 9c8bef3f1c8e0241dc5781ec5408162b24121b43.
- Original consumer environments and indices, 1825 Python paths, 36 unrelated assets/logs and 1253 private measurement files were preserved. git diff --check and zdev check base --format json passed.
