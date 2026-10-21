+++
schema_version = 1
id = "base-113"
key = "shared-expression-traversal"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Own shared expression traversal and adopt extensions in both hosts

## Outcome

Zirql owns traversal of its expression AST; both consumers adopt the current extensions through small domain callbacks instead of maintaining structural walkers.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Zincite crates/zincite-query/src/program/builder.rs map_queries (around line 405), functions.rs lower_primitives/check_values, and Zirium crates/zirium/src/query/functions.rs normalize_with plus report.rs visit_value enumerate Zirql variants. Start with src/value.rs, reader.rs and existing disposal/traversal code. Both consumers currently pin 0.2.0 and require adoption of newer branch variants; this task owns that initial integration. Read the later Complete the common language boundary section too: subsequent tasks move shared stage ownership into core. Keep this traversal slice usable for that representation; it does not freeze the current host wrappers or prepared-session interfaces.

## Boundaries

- Keep MiniZinc/MLIR leaf semantics, scope/effect decisions, native owners and data-edit validation in their hosts. Use small operations for demonstrated clients, not a compiler-pass framework.

## Done when

- [ ] Shared owned payload mapping and bounded borrowed/mutable traversal cover every current Zirql expression and generator form, preserving ranges and order; native payload transformation and host admission remain explicit.
- [ ] Zincite map_queries/map_clauses and structural portions of lowering/native discovery, and Zirium normalization/native-root discovery use the common operations; replaced exhaustive structural matches are removed. Session-selection walks may remain until the prepared-route task.
- [ ] Both consumers compile against the accepted extensions revision and support map/flat_map, selective generators, windows/runs_by, with and align on admitted ordinary report data through their textual and existing builder paths; necessary exhaustive-match changes are included. Data-edit subset restrictions remain enforced.
- [ ] Focused host examples demonstrate ordinary nested traversal and native payload retention, plus adopted extension behavior; shared embedding and consumer support docs describe the actual support and deleted responsibilities.

## Validation

- Reuse existing core and host query controls; add a compact nested-expression/native-payload control and only missing adoption coverage. Exercise inline/file/library paths where affected.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
