# Future publication

The eventual canonical URL is `https://zayenz.se/zirql/`. The intended route is
a separately built static directory imported into the existing personal-site
repository, following Zincite's approach. This document prepares that route;
it does not register a project, import files or authorize deployment.

## Prepare the reviewed build

Choose a reviewed Zirql commit. Check that current-behavior guides and dependency
pins describe it, and that proposed language examples remain visibly labelled.
From the Zirql checkout:

```sh
cd site
npm ci
npm run check
npm run build
npm run preview -- --host 127.0.0.1
```

The default origin and base are `https://zayenz.se` and `/zirql/`. Review the
built preview, including search, links, assets, themes and mobile layout.
`dist/` contains the contents of the future `/zirql/` directory; do not introduce
another `zirql` level while importing it.

## Import through the personal site's release process

When publication is explicitly requested, read the personal site's current
repository instructions and project-import configuration. Confirm or add a
reviewed `zirql` project registration for `/zirql/`; do not assume that it already
exists. Preserve unrelated changes and record the current production commit,
deploy identity and immutable URL as a rollback point.

Zincite's import currently uses `npm run import:project`. After confirming that
the same command and project registration apply, a Zirql import would use:

```sh
npm run import:project -- --project zirql --source /absolute/path/to/zirql/site/dist --dry-run
npm run import:project -- --project zirql --source /absolute/path/to/zirql/site/dist
```

Review dry-run output before importing. For a reviewed replacement of an existing
import, use the importer's explicit `--replace` option. Keep the guides, assets,
Pagefind index and sitemaps together. Let the personal site own its error page.
Do not hand-edit generated files.

Commit the reviewed import in the personal-site repository, recording its Zirql
source revision. Run that repository's required release checks and use its
existing Git-based production route. Check current production policies at that
time; these instructions do not authorize a direct hosting upload or DNS change.

## Verify and retain a rollback

After publication, verify the intended source commit reached production and
check the actual HTTPS paths:

- The homepage and an existing personal-site route still work.
- `/zirql/` and guide pages load; slash redirects follow the site's policy.
- Search opens, finds a guide and follows its result.
- Favicon, styles and Pagefind requests succeed below `/zirql/`.
- Canonical URLs and the sitemap use `https://zayenz.se/zirql/`.
- Mobile navigation, both themes and keyboard focus work; code and tables do
  not widen the page.

Retain source and deploy revisions with the verification result. For rollback,
revert only the import and associated registration changes through the normal
personal-site release route. A first-publication rollback removes the new
project directory. Recheck the homepage and affected routes afterwards.
