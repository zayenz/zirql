+++
schema_version = 1
id = "base-002"
key = "zincite-read-only-adapter"
area = "base"
status = "open"
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

- [ ] Syntax, string, literal and semantic read-only stages run through shared sequencing, filtering and applicable head/count/unique/tally operations.
- [ ] Both zincite query and zincite-query exercise that path and retain existing outputs, source coordinates and incomplete-report status.
- [ ] Zincite parsing can lower existing structures to Zirql without requiring a second MiniZinc parser or making Zincite a core dependency.
- [ ] Host scans and semantic demands retain their separate admission rule while nested shared work consumes the active budget.
- [ ] Attributable integration changes are committed separately in each affected repository and their identifiers are recorded.

## Validation

- Reuse Zincite query and CLI tests; add only a public behavior control for a gap not already covered by the existing native result, emitter or Unknown checks.
- Run required Rust checks in Zirql and Zincite, byte-diff checks and changed-record validation in each owning area.
