+++
schema_version = 1
id = "base-093"
key = "consistent-field-pipeline-routing"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-092"]
+++
# Compose field access with shared pipeline suffixes consistently

## Outcome

Ordinary constructed field pipelines use the shared language without requiring grouping, while native capture continuations retain their domain meanings and native ownership.

## Context

The core5b CSV program [outer.items | filter(_ > 1) for outer in [{"items":[1,2,3]}]] fails with predicate expressions unsupported by this host; grouping only (outer.items) returns [[2,3]]. Zirium integration independently reproduces the same shape inside a named predicate, failing field access requires a native capture. Root and a separate read-only assessment confirmed reader.rs field_pipeline unconditionally routes name.field followed by a pipe into native ExpressionKind::Field before kinds are available. This can fail before lower_value_suffix or host normalization; a host-only workaround cannot fix CSV. Read the brief One shared query reader and Scoped predicate expressions sections. Inspect Reader::atom/field_pipeline/value_suffix/native_tail/value_source, shared field preparation/execution and tests/native_programs.rs native_captures_keep_written_fields_and_feed_shared_projection. Existing value K::Field, value_source and prepared field selection may supply a small shared solution; do not assume retains_native_values alone is a universal native capability flag. Both consumer migrations are active and own their adapters/final pins after this independently accepted correction.

## Boundaries

- Use the existing value/native AST, field selection, scopes, preparation and evaluator. Add no second parser/evaluator, generic grammar configuration, source rewriting, native reflection, serialization fallback or duplicated lexical type inference. Any narrow API adjustment must have a concrete current adapter need.
- Preserve native capture fields, handles, side/owner identity and existing native stage meanings, including m.add | names | one and m.rhs | count. Do not expose constructed wrappers as fake native capture records or convert opaque/native/keyed values into positional lists to mask routing.
- Own core reader/preparation changes strictly necessary for this composition, focused existing-fixture/CSV controls and current language/API docs. No consumer source/pin edits, registry publication, release tags/assets or deployment; Root owns transport, records, final pins and serial consumer acceptance.

## Done when

- [ ] Ungrouped and grouped constructed field prefixes followed by existing common suffixes have the same shared value behavior. The reported CSV field/filter example returns [[2,3]] in both forms, and a named outer capture in a nested predicate retains its scope and actual element subject.
- [ ] Existing native capture field pipelines and native operation continuations remain usable with typed native results and identity/ownership preserved. The reader uses a coherent shared continuation boundary rather than consumer-specific generic parsing; scalar-only readers such as CSV need no native query implementation.
- [ ] Original source ranges, lexical captures, all-written/typed-empty admission, purity and one cumulative work/result/nesting allowance survive the changed route, including located invalid field or predicate refusal and error restoration.
- [ ] Required Rust/MSRV/fatal docs checks and affected site/link/Root desktop/mobile review pass. Document the current shared composition and narrow adapter contract; retain runnable CSV and native capture examples without a grouping-only workaround. Root independently accepts and pushes the actual revision before final consumer pin/verification.

## Validation

- Reuse the isolated grouped/ungrouped CSV pair and a nearby public native fixture for one named nested field-filter body; add focused assertions only for this concrete routing defect. Preserve existing native capture names/count continuation controls and native identity/owner tests; no syntax matrix or separate cross-consumer replay campaign.
- Use a typed empty field source and a located invalid field/predicate or low-limit reentry refusal to check unchanged admission and budgets. Inspect both actual consumer call sequences read-only; their source/runtime integration remains in base087/base089.
- Run cargo fmt --all -- --check; fatal workspace/all-target Clippy; workspace tests; Rust1.88 tests; warning-fatal docs/doctests; git diff --check and zdev check base --format json. Run affected site check/build/local links and Root desktop/mobile review. Package contents or examples changed by the fix are checked proportionally; final combined registry/installation/CI acceptance remains084.
