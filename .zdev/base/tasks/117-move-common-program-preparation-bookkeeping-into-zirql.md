+++
schema_version = 1
id = "base-117"
key = "shared-preparation-session"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-116", "base-114", "base-125", "base-126", "base-127"]
+++
# Move common program preparation bookkeeping into Zirql

## Outcome

Both hosts retain domain stage handling while Zirql manages common stages/calls, prepared entries, bindings and program scope progression for native and constructed clients.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Compare Zincite program/functions.rs Session (around 605) and Zirium query/functions.rs Session (around 827) with src/functions.rs PreparedFunctions/PreparedPatternBinding and src/program.rs. The preceding Zirium task removes its competing report route. Read Complete the common language boundary in brief.md. The new common-stage, typed-native-call and program-reader tasks settle the representation before this extraction. Extend existing preparation rather than adding a second evaluator or cache; CSV’s following task must be able to consume it without implementing native host traits.

## Done when

- [x] Zirql owns request-local prepared-entry storage and shared prepare/bind/run bookkeeping; common statement scope progression is shared through the lowered program using the common program reader and core-owned stages/calls. Host stage argument/kind hooks remain only for native operations.
- [x] Both host Session implementations shrink to domain adaptation; duplicate maps, lookups and shared binding progression are removed rather than hidden behind new wrappers.
- [x] Statement-order bindings, parameter positions, pattern atomicity, nested relative inputs, native owners, preflight effects and continuing budgets remain observable through existing public programs.
- [x] The shared prepared path can combine named functions, parameters and ordered pattern bindings with constructed current input and shared bindings under one continuing budget. Function bodies still receive data explicitly; do not fall back to NoQueries in a way that drops entry-program input or sharing. The following constructed embedding task supplies the concrete adapter and CSV/words adoption.
- [x] Embedding documentation shows a minimal data/function/operator host and names any remaining host orchestration with its concrete domain reason.

## Validation

- Reuse both hosts’ saved binding/function/pattern/projection programs and failure buffering controls; add only a demonstrated missing scope case.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.

## Result

PreparedProgram owns request-local prepared entries and ordered scope/run progression through the shared driver. Removed both host Sessions, maps and range aliases; retained native ownership, domain admission, Unknown/edit safety and emission. Canonical Zirql f89cb7a68a6124e0938ea582e03efc040b5ecafe; Zincite 93c45ea588ac95ff16c68c307536f496e5cbf48f; Zirium 918ae1a0e52ed92629a1b64e634f1f8e170c8865.

Validation:

- Independent complete review PASS without corrections at Wc3359ea9ddccfd59; post-review core and all four consumer Git fields equal, final core comparison equal before completion.
- All three required Rust fmt/Clippy/workspace suites pass; Zirium all-target/all-feature single-thread tests, doctests and warnings-denied docs pass. Rebuilt Python 179 passed, 2 skipped; Ruff and ty pass.
- Compiled embedding scope/emission control and existing query_profile smoke pass. All three site checks/builds and production-boundary local links pass: 6455/1238/891.
