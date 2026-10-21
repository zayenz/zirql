+++
schema_version = 1
id = "base-122"
key = "canonical-syntax-asset"
area = "base"
status = "done"
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

- [x] Zirql owns the complete common highlighting asset and a small explicit revision-pinned sync/check procedure; both consumer copies are updated and their source revision is recorded.
- [x] Host overlays contain only host distinctions; redundant common patterns are removed and intentional comment/quoted-name/numeric differences are retained explicitly.
- [x] All three sites build independently from checked-in assets; contributor documentation describes the one update path and distinguishes highlighting from host capability support.

## Validation

- Inspect highlighting of a compact shared/host example set through existing site tooling, run JSON/JavaScript checks and affected site builds plus git diff --check. No new Rust/product tests.
- Run zdev check base --format json for records and record separate host commits.

## Result

Shared canonical highlighting and revision-pinned sync/check procedure adopted by both independent host sites. Common grammar and local overlays are separated; legacy highlighting allowances are documented accurately. Zincite commit f72276153852481b479417b470689df53c7ff772; Zirium commit 13ed3fd6ec6326239d990987d8ff6ea0db553a04. Canonical source revision ab500fadfc5365c55018a63947ae698a586640ca retained.

Validation:

- Independent full review, accepted comment correction and focused recheck passed; unchanged W3c83ac777b54b884 snapshot.
- All three site check/build commands passed; Shiki scopes, sync roundtrip/mismatch, JSON/JS syntax and diff checks passed.
- Native handles, runtime semantics and host capability policy remain host-owned; no runtime changes or new product tests.
