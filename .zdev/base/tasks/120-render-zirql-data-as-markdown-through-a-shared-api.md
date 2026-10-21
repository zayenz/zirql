+++
schema_version = 1
id = "base-120"
key = "shared-markdown-data"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-118"]
+++
# Render Zirql data as Markdown through a shared API

## Outcome

Zirql provides bounded Markdown rendering for ordinary data and CSV offers a documented rendering choice.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Follow the brief’s Markdown contract. Zirium query/render.rs markdown/markdown_map/header/row/escape provides the existing table layouts and escaping. Reuse the shared JSON codec for nested cells and structural keys. Add a small CSV output-format option using its existing CLI parsing, not a new CLI framework.

## Boundaries

- Core renderer and CSV only; consumer adoption follows separately. No HTML, configurable templates or automatic rendering during evaluation.

## Done when

- [x] The common renderer supports scalars, scalar collections, records/dictionaries, record-row sequences and keyed row maps with stable first-seen columns, missing/absent blank cells, escaped content and explicit empty results.
- [x] Nested cells and structural keys use compact shared JSON; nominal/keyed data remain representable, while computational values receive a located refusal at the bridge.
- [x] Table rectangular expansion, scalar conversion and output bytes share one request budget, with checks before known allocations and no partial buffered result on failure.
- [x] CSV can render a complete query result as Markdown; Rust API docs, library entry/sidebar and runnable table examples document shape, escaping and budget behavior.

## Validation

- Reuse/adapt the meaningful Zirium table examples; check mixed columns, nested cells, empty data, Markdown/HTML-sensitive text and bounded rectangular expansion using a few focused cases.
- Run Zirql Rust checks and the documented CSV Markdown example; git diff --check, zdev check and docs content/link checks.

## Result

Added bounded Markdown rendering with stable record columns, nested JSON cells, explicit empty results, paragraph escaping and CSV output format selection; documented Rust API, reference/sidebar and CSV examples.

Validation:

- Independent full review plus one accepted escaping correction and fresh focused recheck passed; snapshot W578ba9c5ef1198c6 compared equal.
- cargo fmt --all -- --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace passed.
- Documented CSV Markdown and scalar paragraph controls, Astro check/build, generated content/links, git diff --check and zdev check passed.
