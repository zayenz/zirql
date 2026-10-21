+++
schema_version = 1
id = "base-121"
key = "integrate-shared-markdown"
area = "base"
status = "done"
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

- [x] Zirium’s existing markdown stage calls the shared renderer and its replaced table/escaping code is removed, retaining established scalar/map layouts and adding supported common row/nested-cell forms.
- [x] Zincite read-only report programs support an explicit markdown emitter through the existing emission mechanism; source/data-edit output remains separate and unsupported edits are rejected before execution.
- [x] Both hosts demonstrate library and inline/file Markdown reports, including native subjects projected to ordinary rows, with stable ordering, correct escaping and unchanged later-error buffering.
- [x] Host docs provide one domain recipe each, list actual support and link the shared Markdown library contract.

## Validation

- Reuse existing Zirium Markdown fixtures and add one Zincite report/terminal-emitter control plus later-failure/limit checks where missing; avoid a format matrix.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.

## Result

Added shared typed mixed-report Markdown and adopted it in both consumers, removing Zirium ordinary table/cell loops while retaining native diff tables, framing and buffers. Zincite has a terminal Program emitter, is_markdown and pre-execution edit refusal. Added runnable recipes and complete API/index/sidebar/support docs. Zincite 9dc84523c15235d6aaa6a26dfce52e48538ddc1d; Zirium 026f2fee71ff8c53ddeb71e51dc3816d4e6b3fdb; canonical Zirql pin 843190701089ef2d1644cea8dc0abe92f080a63b.

Validation:

- Independent full review and one focused correction recheck PASS Wa95bde0781dcf9b4; core and both complete consumer states unchanged. All three Rust formatting, fatal Clippy, workspace tests and site check/build passed, including Zirium required all-feature single-thread/doc checks. Focused exact edit-then-Markdown refusal, explicit source edits, read-only tables, native rows, inline/file/library paths, limits and later-error buffering passed.
