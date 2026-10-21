+++
schema_version = 1
id = "base-121"
key = "integrate-shared-markdown"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-119", "base-120"]
+++
# Adopt shared Markdown output in both consumers

## Outcome

Zirium delegates table rendering to Zirql and Zincite exposes the same Markdown output for read-only report data.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Start with Zirium query/render.rs and query/program.rs Markdown stage validation/dispatch, Zincite program.rs Emitter/parse/evaluate and CLI read-only reporting. Reuse the JSON integration’s native projection/container bridge rather than introducing a second traversal. Shared data formatting belongs to Zirql; host source output and envelopes remain explicit.

## Done when

- [ ] Zirium’s existing markdown stage calls the shared renderer and its replaced table/escaping code is removed, retaining established scalar/map layouts and adding supported common row/nested-cell forms.
- [ ] Zincite read-only report programs support an explicit markdown emitter through the existing emission mechanism; source/data-edit output remains separate and unsupported edits are rejected before execution.
- [ ] Both hosts demonstrate library and inline/file Markdown reports, including native subjects projected to ordinary rows, with stable ordering, correct escaping and unchanged later-error buffering.
- [ ] Host docs provide one domain recipe each, list actual support and link the shared Markdown library contract.

## Validation

- Reuse existing Zirium Markdown fixtures and add one Zincite report/terminal-emitter control plus later-failure/limit checks where missing; avoid a format matrix.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
