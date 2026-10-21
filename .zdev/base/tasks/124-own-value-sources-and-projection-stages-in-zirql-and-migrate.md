+++
schema_version = 1
id = "base-124"
key = "core-value-projection-stages"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-116"]
+++
# Own value sources and projection stages in Zirql and migrate both hosts

## Outcome

Shared value sources and projection are represented, prepared and executed by Zirql; both consumers remove their corresponding stage wrappers and dispatch.

## Context

Read brief.md sections Consolidate language machinery and data rendering (2026-10-21) and Complete the common language boundary (2026-10-21), including their compatibility, documentation and validation contracts. src/program.rs ExpressionStep currently has only opaque Stage and Emit alternatives. src/reader.rs asks hosts to manufacture projection stages. Compare Zincite program.rs Stage::FunctionValue/FunctionSource/RecordValue/Projection and program/composition.rs argument/type/execution arms with Zirium query/model.rs FunctionValue/Comprehension/Projection and query/report.rs. Start with src/program.rs, projection.rs and functions.rs; reuse the prepared route established by base-116 and the traversal API from base-113.

## Boundaries

- Migrate ordinary value sources, constructed record expressions and projection only; later shared-stage work owns matching, ordering and iterative bodies.
- Retain native leaf payloads, host subject/kind views, ownership and effect admission. Keep typed builders and callable paths usable without parsing or rendering.

## Done when

- [ ] The existing core representation directly owns these common forms and their prepared arguments, type discovery and execution; hosts do not manually redispatch a new common-stage wrapper.
- [ ] Both hosts use the shared forms and delete superseded variants, reader factories and common argument/type/execution branches; any retained wrapper has a concrete domain responsibility recorded in the result.
- [ ] Nested native/constructed projections retain lexical scope, relative input, source locations, native owners, declared empty shapes and cumulative limits; host-specific emission and edits retain their contracts.
- [ ] Rust embedding docs show the new core/native boundary; both consumer architecture/support guides and relevant projection reference links describe actual adopted behavior.

## Validation

- Reuse public native projection, constructed record, saved-binding and typed-builder controls in both hosts; add only a missing relative-input or declared-empty case. Reuse base-116 measurement evidence unless this change gives a concrete reason to measure again.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.
