+++
schema_version = 1
id = "base-112"
key = "library-reference-sidebar"
area = "base"
status = "open"
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

- [ ] A library overview and complete alphabetical index link every supported shared function, pipeline operation and operator to a detailed entry.
- [ ] Reference entries are grouped by purpose, with signatures/forms, accepted inputs/results, concise examples, empty/absence/error/order semantics where relevant and host-support caveats; links reuse existing deeper guides.
- [ ] The site sidebar has an organized Library reference section with collapsible purpose groups and navigable function/operator entries; tutorials, language grammar and Rust API remain easy to find.
- [ ] Content preparation includes all reference pages and rewrites their internal links correctly; existing guide links remain valid.
- [ ] Inventory is checked against shared reader/function/operator definitions and existing documented special forms so new extensions and aliases are included without inventing supported syntax.

## Validation

- git diff --check
- zdev check base --format json
- Run existing site content preparation and inspect generated navigation/pages and internal reference links; no new product tests or Rust checks.
