+++
schema_version = 1
id = "base-122"
key = "canonical-syntax-asset"
area = "base"
status = "open"
complexity = "routine"
afk = true
priority = "normal"
blocked_by = []
+++
# Synchronize one canonical Zirql highlighting asset with host overlays

## Outcome

Common highlighting changes are authored in Zirql and deliberately synchronized to both independent host sites.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. All projects carry assets/syntax/zirql.tmLanguage.json. Zincite overlays it in site/src/zirql.mjs and Zirium in site/astro.config.mjs; their base copies already differ. Review common current forms, including case/with/control forms, against the actual common reader. An overlay can retain host lexical differences without claiming evaluator support.

## Done when

- [ ] Zirql owns the complete common highlighting asset and a small explicit revision-pinned sync/check procedure; both consumer copies are updated and their source revision is recorded.
- [ ] Host overlays contain only host distinctions; redundant common patterns are removed and intentional comment/quoted-name/numeric differences are retained explicitly.
- [ ] All three sites build independently from checked-in assets; contributor documentation describes the one update path and distinguishes highlighting from host capability support.

## Validation

- Inspect highlighting of a compact shared/host example set through existing site tooling, run JSON/JavaScript checks and affected site builds plus git diff --check. No new Rust/product tests.
- Run zdev check base --format json for records and record separate host commits.
