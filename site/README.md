# Documentation site

Authoritative reader guides live in `../docs/`. The preparation script adds
Starlight metadata and rewrites their repository-relative links into site links.
Generated content is ignored. Edit the Markdown guides, then rerun preparation;
site tooling remains independent of Cargo. The sidebar's Start here section
contains word analysis, its Rust setup and the [CSV usage guide](../docs/csv.md).
[Build the CSV host](../docs/csv-host.md) is in the Implementation reference
section. Existing `/guides/` routes remain available.

The landing page uses an array of words to demonstrate adjacent pairs,
selection and summaries. The preparation script turns its marked example
sections into Starlight tabs while keeping `docs/index.md` readable in the
repository. The CSV guide uses the same conversion for groups marked with
`<!-- tabs:start -->` and `<!-- tabs:end -->`; each `###` heading becomes a tab.
These pages are generated as MDX to use Starlight's accessible tab components.
The CSV examples separate highlighted queries, input data, formatted results and
copyable shell commands. Their tables scroll within the content on narrow
screens. Keep commands multiline in the source so they remain readable in both
the repository and the rendered tabs.
`examples/words.rs` runs the word expressions with the common reader.
The adapter guide uses Zincite and Zirium to explain native identity and host
policy; commands and consumer workflows belong on their own sites.

The selected Zirql logo supplies the navy, royal blue and pale steel palette.
The header uses transparent SVGs from `../assets/logo/vector-v1/`, with pale
lettering in the dark theme and navy lettering in the light theme. Both retain
the blue accent. The asset directory includes all five layouts in both themes
as SVG and PNG. Colors and typography live in `src/styles/custom.css`.
Starlight supplies theme switching, search and mobile navigation.

## Syntax highlighting

Use `zirql` on fenced query examples; keep output blocks labelled with their
output format. `../assets/syntax/zirql.tmLanguage.json` supplies the TextMate
grammar registered with Starlight's Shiki highlighter. Other documentation sites
can register the same JSON grammar to render Zirql snippets consistently.

The grammar highlights language keywords, operators, types, literals, comments
and calls. Domain operation names are highlighted as calls without a separate
list for each host. Highlighting is lexical; parsing and host validation decide
whether a query is accepted.

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
origin `https://zayenz.se` and base `/zirql/`. Building does not update the
published site; publication requires a separate reviewed import. Pagefind search needs the built preview.

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
under `/zirql/`. For CSV changes, check `/guides/csv/` and `/guides/csv-host/`
under that base, including the input, result and command tabs. Follow a guide
link and a Pagefind search result. Check light and dark themes, narrow mobile
navigation, keyboard focus and Escape to dismiss
search. Code and tables should scroll within the content. Confirm favicon,
styles, search assets, canonical URLs and sitemap paths use `/zirql/`.

No new product tests are needed for these guides. Repository changes also run
the checks listed in `../AGENTS.md`. Follow the [publication procedure](PUBLISHING.md)
to import a reviewed build into zayenz.se and publish it through the personal
site's Git deployment.
