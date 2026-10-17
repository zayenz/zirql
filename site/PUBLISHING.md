# Publish the documentation site

The canonical URL is `https://zayenz.se/zirql/`. Build this site separately and
import its static output into the personal site's `public/zirql/` directory.
The personal site owns the project card, sitemap discovery and deployment.

Production uses the existing Netlify project `zayenz-web`
(`36755581-737f-42cb-ac2f-a16d37fcd00f`). A push to the personal site's `main`
branch triggers its hosted release build. Non-Git production deployments are
blocked. Keep the existing build command, functions, environment and domains.

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
`dist/` contains the contents of the `/zirql/` directory; do not introduce
another `zirql` level while importing it.

## Import through the personal site's release process

When publication is explicitly requested, read the personal site's current
repository instructions and project-import configuration. Confirm or add a
reviewed `zirql` project registration for `/zirql/`; do not assume that it already
exists. Preserve unrelated changes and record the current production commit,
deploy identity and immutable URL as a rollback point.

The shared registry in the personal site's `src/data/projects.ts` supplies the
project card, import destination and sitemap discovery. Its Zirql entry needs
`site: '/zirql/'`, `importedSite: true`, the selected logo and guide links.
Register it together with the first import. From that repository, run:

```sh
npm run import:project -- --project zirql --source /absolute/path/to/zirql/site/dist --dry-run
npm run import:project -- --project zirql --source /absolute/path/to/zirql/site/dist
```

Review dry-run output before importing. For a reviewed replacement of an existing
import, use the importer's explicit `--replace` option. Keep the guides, assets,
Pagefind index and sitemaps together. Let the personal site own its error page.
Do not hand-edit generated files.

Commit the reviewed import in the personal-site repository, recording its Zirql
source revision. Run `npm run release:check` with that repository's Node.js 24
toolchain, then publish through its Git-based production route. Source pushes to
Zirql do not update the imported site; each update requires a new build and import.

## Verify and retain a rollback

After publication, verify the intended source commit reached production and
check the actual HTTPS paths:

- The homepage and an existing personal-site route still work.
- `/zirql/` and guide pages load; slash redirects follow the site's policy.
  Include `/zirql/guides/csv/` and `/zirql/guides/csv-host/` when importing CSV
  guide changes, and check that the example tabs and tutorial links work.
- Search opens, finds a guide and follows its result.
- Favicon, styles and Pagefind requests succeed below `/zirql/`.
- Canonical URLs and the sitemap use `https://zayenz.se/zirql/`.
- Mobile navigation, both themes and keyboard focus work; code and tables do
  not widen the page.

Retain source and deploy revisions with the verification result. For rollback,
revert only the import and associated registration changes through the normal
personal-site release route. A first-publication rollback removes the new
project directory. Recheck the homepage and affected routes afterwards.
