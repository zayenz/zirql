+++
schema_version = 1
id = "base-057"
key = "share-template-computed-key-construction"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Share pure interpolation and computed object-key construction

## Outcome

Zirql supplies reusable pure template expansion and computed string-key construction, and Zirium delegates its existing general behavior to those operations.

## Context

Zirium query/render.rs::interpolate expands parsed PrintPart fragments; query/literal.rs::evaluate constructs nested JSON literals, inserts saved binding snapshots and rejects keys that collide after interpolation. query/program.rs preflights those bindings and kinds. query/diff_program.rs has another interpolation loop with different brace/newline details. Core strings, concatenation and static records cover only the non-template subset. Base-051 owns correlated map_by, not written object templates. Read those implementations, Zirium docs/query-language.md JSON literals and interpolation, and its existing cli.rs, cli_diff.rs and query_dsl.rs controls before designing the small shared interface.

## Boundaries

- Preserve existing Zirium syntax, output ordering/shapes, ordinary-versus-diff brace behavior, binding scope, preflight, errors and print newlines. Common reader strings remain literal; no new implicit interpolation syntax, Zincite syntax, template engine, formatting registry or broad coercion policy.
- Keep native inspection/snapshot conversion, JSON numeric admission and escaping, Markdown/rendering and emission with Zirium. Legacy JSON numbers outside i64 and null remain valid; do not force them through ConstructedValue or stringify native payloads.

## Done when

- [ ] Shared callable operations expand text fragments and assemble computed string-key entries using the current context and budget. Written evaluation order and duplicate-key rejection before evaluating that entry's value are explicit; failures propagate without partial successful results.
- [ ] Zirium ordinary print and nested literal strings/keys, plus diff print expansion, delegate reusable computation to Zirql. Frontends may retain their existing parsing and host-specific binding-to-text admission (count or exactly one string); host conversions retain saved JSON snapshot semantics and native identity where values stay native.
- [ ] A compact non-MLIR caller can compose shared interpolation with shared object construction without writing its own expansion or duplicate-key loop. The implementation reuses existing construction facilities where applicable and does not introduce a parallel general evaluator.
- [ ] Preflight still rejects unavailable bindings and known unsupported interpolation kinds before effects. Expansion bytes, entry storage, nesting and copies consume the active limits; diagnostics retain available query locations. Existing output buffering and later-error behavior survive the migration.

## Validation

- Reuse existing nested/interpolated literal, escaped-brace, duplicate-key, invalid-binding/kind, print/diff and empty-stdout-on-later-error controls. Include the legacy large JSON integer and saved snapshot behavior. Add only a compact shared-library composition control and focused resource failure where existing coverage is insufficient.
- Run the brief-required Zirql Rust checks and affected Zirium Rust/query/CLI checks, git diff --check and zdev check base --format json. Record separate consumer commits and preserve unrelated work. No publication, benchmark framework, internal call-count assertions or generic test matrix.
