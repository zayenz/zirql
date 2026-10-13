# Documentation site

Authoritative reader guides live in `../docs/`. The preparation script adds
Starlight metadata and rewrites their repository-relative links into site links.
Generated content is ignored. Edit the Markdown guides, then rerun preparation;
site tooling remains independent of Cargo. The sidebar separates language
reference, worked examples and Rust embedding guides; all retain `/guides/`
routes.

The selected Zirql logo supplies the navy, royal blue and pale steel palette.
Its horizontal logo and glyph are copied unchanged into `public/`; the header
clips surrounding whitespace in CSS. A white header keeps the original PNG
readable in both themes. Colors and typography live in `src/styles/custom.css`.
Starlight supplies search and mobile navigation.

## Build and preview

Use Node.js 22.12 or later and npm. From the repository root:

```sh
cd site
npm ci
npm run check
npm run build
npm run preview -- --host 127.0.0.1
```

Open `/zirql/` at the origin printed by Astro. The default build uses canonical
origin `https://zayenz.se` and base `/zirql/`. This metadata reserves the eventual
URL; building does not publish anything. Pagefind search needs the built preview.

`npm run dev` starts the editing server. Restart it after editing a guide outside
`site/` so the preparation step runs again. No Rust build invokes npm.

For another local base, set the same values for build and preview:

```sh
SITE_URL=https://zayenz.se SITE_BASE=/zirql/ npm run build
SITE_URL=https://zayenz.se SITE_BASE=/zirql/ npm run preview -- --host 127.0.0.1
```

Output is in ignored `dist/`. Generated Markdown is in ignored
`src/content/docs/`; Astro caches and installed dependencies are also ignored.
The lockfile pins the website toolchain independently of the Rust dependency.

## Check the rendered site

Run `check` and `build`, then review the landing page and representative guides
under `/zirql/`. Follow a guide link and a Pagefind search result. Check light
and dark themes, narrow mobile navigation, keyboard focus and Escape to dismiss
search. Code and tables should scroll within the content. Confirm favicon,
styles, search assets, canonical URLs and sitemap paths use `/zirql/`.

No new product tests are needed for these guides. Repository changes also run
the checks listed in `../AGENTS.md`. Publication has a separate review and the
[future publication procedure](PUBLISHING.md); this task does not deploy.
