+++
schema_version = 1
id = "base-117"
key = "shared-preparation-session"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-116", "base-114"]
+++
# Move common program preparation bookkeeping into Zirql

## Outcome

Both hosts retain domain stage handling while Zirql manages shared prepared entries, bindings and program scope progression.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Compare Zincite program/functions.rs Session (around 605) and Zirium query/functions.rs Session (around 827) with src/functions.rs PreparedFunctions/PreparedPatternBinding and src/program.rs. The preceding Zirium task removes its competing report route. Extend existing preparation rather than adding a second evaluator or cache.

## Done when

- [ ] Zirql owns request-local prepared-entry storage and shared prepare/bind/run bookkeeping; common statement scope progression is shared through the lowered program with explicit host stage argument/kind hooks.
- [ ] Both host Session implementations shrink to domain adaptation; duplicate maps, lookups and shared binding progression are removed rather than hidden behind new wrappers.
- [ ] Statement-order bindings, parameter positions, pattern atomicity, nested relative inputs, native owners, preflight effects and continuing budgets remain observable through existing public programs.
- [ ] Embedding documentation shows a minimal data/function/operator host and names any remaining host orchestration with its concrete domain reason.

## Validation

- Reuse both hosts’ saved binding/function/pattern/projection programs and failure buffering controls; add only a demonstrated missing scope case.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
