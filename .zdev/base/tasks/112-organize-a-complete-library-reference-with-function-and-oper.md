+++
schema_version = 1
id = "base-112"
key = "library-reference-sidebar"
area = "base"
status = "done"
complexity = "standard"
afk = true
blocked_by = []
+++
# Organize a complete library reference with function and operator navigation

## Outcome

Readers can find every currently supported shared Zirql function, pipeline operation and operator in a structured library reference and its sidebar.

## Context

The user requested a well structured library reference with good sidebar organization of all functions and operators. Current site/astro.config.mjs lists broad language guides and docs/language-reference.md has only a partial built-in table. Use reader/runtime source and existing guides to inventory supported operations, including feature/extensions additions. Keep tutorials available and distinguish host-dependent native operations from shared constructed operations. Read the brief's documentation and testing guidance.

## Boundaries

- Documentation and necessary site content/navigation wiring only; preserve language/runtime behavior and existing guide URLs. No consumer changes, publication or deployment.
- Reference supported current branch behavior only; CP and unimplemented design forms are not supported library entries. Host-specific operations stay with their host guides.

## Done when

- [x] A library overview and complete alphabetical index link every supported shared function, pipeline operation and operator to a detailed entry.
- [x] Reference entries are grouped by purpose, with signatures/forms, accepted inputs/results, concise examples, empty/absence/error/order semantics where relevant and host-support caveats; links reuse existing deeper guides.
- [x] The site sidebar has an organized Library reference section with collapsible purpose groups and navigable function/operator entries; tutorials, language grammar and Rust API remain easy to find.
- [x] Content preparation includes all reference pages and rewrites their internal links correctly; existing guide links remain valid.
- [x] Inventory is checked against shared reader/function/operator definitions and existing documented special forms so new extensions and aliases are included without inventing supported syntax.

## Validation

- git diff --check
- zdev check base --format json
- Run existing site content preparation and inspect generated navigation/pages and internal reference links; no new product tests or Rust checks.

## Result

Added a complete library reference, ten purpose categories, alphabetical index and direct collapsible sidebar links for supported functions, operators and shared forms.

Validation:

- Independent verification passed against W602dd2965eaf346d; snapshot unchanged.
- git diff --check, zdev check base --format json, content preparation, JavaScript syntax and inventory/link audits passed; 119 sidebar links verified for three site base paths. Astro rendering not run: site dependencies absent.
