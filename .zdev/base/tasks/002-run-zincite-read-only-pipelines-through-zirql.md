+++
schema_version = 1
id = "base-002"
key = "zincite-read-only-adapter"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-001", "base-011"]
+++
# Run Zincite read-only pipelines through Zirql

## Outcome

Existing Zincite read-only query library and CLI paths use Zirql sequencing and common stream operations with native CST, literal and semantic results.

## Context

Read Zincite AGENTS.md, its area brief and query language.rs/lib.rs, inspection.rs, semantic.rs, structured.rs and queries/CLI tests. Consume the live public semantic owner; its in-progress implementation is unrelated work. Before edits, establish both repository baselines and reconcile base-126's framework ownership in its context without claiming its program outcome is done. Begin by lowering the current fixed pipeline; preserve host projections and navigation.

## Boundaries

- Preserve Zincite public query entry points, emitter terminality, result types, source bytes, Unknown semantics, exit codes and current limits.
- This task covers read-only queries; existing transformations retain their established path until the data integration.
- Do not implement a semantic owner or modify unrelated Zincite work. Live public APIs must be available before integration; document an actual missing prerequisite rather than adding stubs.

## Done when

- [x] Syntax, string, literal and semantic read-only stages run through shared sequencing, filtering and applicable head/count/unique/tally operations.
- [x] Both zincite query and zincite-query exercise that path and retain existing outputs, source coordinates and incomplete-report status.
- [x] Zincite parsing can lower existing structures to Zirql without requiring a second MiniZinc parser or making Zincite a core dependency.
- [x] Host scans and semantic demands retain their separate admission rule while nested shared work consumes the active budget.
- [x] Attributable integration changes are committed separately in each affected repository and their identifiers are recorded.

## Validation

- Reuse Zincite query and CLI tests; add only a public behavior control for a gap not already covered by the existing native result, emitter or Unknown checks.
- Run required Rust checks in Zirql and Zincite, byte-diff checks and changed-record validation in each owning area.

## Result

Run Zincite read-only query library and both CLI paths through Zirql while preserving native results, errors, source bytes, limits and the existing edit route. Core pinned at 8a2c11a602affe41bcc87b70a8aaad5b33aa0d2c; consumer integration committed separately at 77e5a91d05f63f932933e75983b0891dc263ad8e (source transport c789d21cd41350eacd82037e3aa9c518a429dc2a). Add the user-approved BAML design reference.

Validation:

- Independent whole-task source verifier PASS at W4dd5c165bfcd9191; no findings or validation writes, coordinator comparison equal immediately before completion.
- Independent canonical Zincite destination verifier PASS at W5ebf6d6e6222bc23; exact nine-path candidate preserved, plus reviewed open base-126 context only. Programs/report outcomes and attended data-edit gate remain pending.
- Required cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace and git diff --check pass independently in both repositories: ten native controls and README doctest; thirty query, eight standalone query CLI and ten root CLI controls.
- Both authoritative zdev check base --format json checks pass. Manifest/Cargo.lock pin the exact private HTTPS Git commit without local overrides; canonical integration pushed and clean, source content equals reviewed transport.
