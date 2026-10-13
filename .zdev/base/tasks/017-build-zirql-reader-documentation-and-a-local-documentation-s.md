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

## Validation

- Run git diff --check and zdev check base --format json.
- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, and cargo test --workspace.
- Run site npm run check and npm run build; preview /zirql/ and check search, links, mobile navigation, themes, assets and canonical URLs.

## Result

Built and independently verified eight reader guides and the local Astro/Starlight site, revised for plain prose and the selected Zirql logo. No deployment.

Validation:

- npm check/build and generated links, assets, canonical URLs and sitemap checks passed.
- Browser checks passed for search/results, mobile navigation, themes, keyboard dismissal and 390px overflow.
- Pinned getting-started example compiled and printed Integer(42); cargo fmt/clippy/test, git diff --check and zdev check passed.
