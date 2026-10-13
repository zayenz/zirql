+++
schema_version = 1
id = "base-017"
key = "docs-site"
area = "base"
status = "done"
complexity = "standard"
afk = true
blocked_by = []
+++
# Build Zirql reader documentation and a local documentation site

## Outcome

Reader guides and a locally reviewable Astro/Starlight site are ready for eventual publication at https://zayenz.se/zirql/.

## Context

The user explicitly requested implementation now, independently of ongoing language work, using zayenz-writing-style and unslop. Use the Documentation site section of the brief and the Zincite site as references. Explain the approximate planned language direction without making delivery promises. Current Rust APIs and examples must remain accurate.

## Boundaries

- Do not deploy, push, or modify the personal-site or consumer repositories.
- Preserve ongoing changes to README.md, src/, tests/ and existing planning records. Implement this task in new docs/ and site/ paths.
- Keep authoritative reader guides in docs/ and generate site copies. No new product tests.
- Use the selected logo's navy, blue and extraction motif to guide the visual language. The logo chat owns logo assets; use its selected variants rather than inventing a separate identity.
- Apply zayenz-writing-style and unslop throughout. Prefer concrete queries and results to abstract slogans, repeated positioning statements or generic landing-page sections.

## Done when

- [x] Reader guides cover getting started, shared concepts, native programs and values, host integration, diagnostics and limits, with current behavior distinguished from design direction.
- [x] A Zirql-branded Astro/Starlight site renders the guides at /zirql/ with navigation, search, responsive layouts and light/dark themes.
- [x] The site has local build/preview instructions and a documented future publication route, without deployment.
- [x] Site checks/build, representative browser checks, and required repository checks pass.
- [x] Navigation separates language reference, examples and embedding. Examples are prominent on the homepage, and the embedding guide explains the dependencies and host responsibilities needed for an integration.

## Validation

- Run git diff --check and zdev check base --format json.
- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, and cargo test --workspace.
- Run site npm run check and npm run build; preview /zirql/ and check search, links, mobile navigation, themes, assets and canonical URLs.

## History

Reopened on 2026-10-13 for the user's requested separation of language reference,
examples and embedding guides. Retain the selected logo and plain writing style.

### Previous completion

Built and independently verified eight reader guides and the local Astro/Starlight site, revised for plain prose and the selected Zirql logo. No deployment.

Validation:

- npm check/build and generated links, assets, canonical URLs and sitemap checks passed.
- Browser checks passed for search/results, mobile navigation, themes, keyboard dismissal and 390px overflow.
- Pinned getting-started example compiled and printed Integer(42); cargo fmt/clippy/test, git diff --check and zdev check passed.

## Result

Reorganized the docs into language reference, worked examples and Rust embedding, with homepage examples and explicit integration requirements. No deployment.

Validation:

- Independent review PASS; git diff --check and zdev check passed.
- Site check/build and internal links/fragments/canonical/assets checks passed; 11 pages built.
- Three local host example commands returned the documented results; browser navigation/search/keyboard/themes/mobile checks passed.
- cargo fmt, clippy and workspace tests passed.

## Publication follow-up

On 2026-10-13, the user authorized publication after this task's local-site
completion. The original boundaries and results above describe that earlier
local delivery.

The reviewed word-model docs were published at https://zayenz.se/zirql/ from
Zirql source commit `7bec181`, imported through personal-site commit `7a7a6130`.
Netlify deployment `6acdfe1ab642bbf05e12e6e0` reached production and was verified
over HTTPS.

The homepage, worked examples and design sketches use words independently of
the consumer dialects. A separate semantic-model integration guide explains
Zincite and Zirium. Selected-logo variants are available as transparent SVGs
and PNGs in both themes; the docs header switches variants and the Projects
card uses the dark SVG.

Validation passed for the source site's check/build, Rust formatting, Clippy,
workspace tests, `git diff --check` and `zdev check base`. The word pipelines
and constructed values were executed through Zirql, and the integration
commands through both consumer CLIs. The personal-site release check passed.
Browser and live HTTPS checks covered navigation, search results, both themes,
mobile layout, assets, canonical URLs, sitemaps and existing personal-site routes.

Later language changes are outside this published snapshot and need a reviewed
build and explicit import before they reach the site.
