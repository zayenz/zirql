# Release Zirql 0.1.0

The 0.1.0 candidate is local preparation until crates.io publication and the
`v0.1.0` tag are explicitly authorized. The registry dependency and versioned
checkout commands in the reader guides describe the release destination;
prepublication checks use the local archive. A path dependency proves that
archive's API, not registry availability.

## Check the source and archive

Use the reviewed release commit with a clean working tree. Keep its commit ID,
the validation logs and the SHA-256 of `target/package/zirql-0.1.0.crate` with the
release artifacts. An archive made with `--allow-dirty` is a draft and must be
recreated from the accepted commit.

```sh
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --locked -- -D warnings
cargo +stable test --workspace --locked
cargo +1.88.0 test --workspace --locked
RUSTDOCFLAGS='-D warnings' cargo +stable doc --no-deps --locked
RUSTDOCFLAGS='-D warnings' cargo +stable test --doc --locked
cargo +stable package --list --locked
cargo +stable package --locked
shasum -a 256 target/package/zirql-0.1.0.crate
```

Compare package lists before and after installing the site's locked dependencies.
The archive must include `src`, the existing tests, both example sources, the
CSV fixtures, reader guides, this procedure, the changelog and both licenses.
It must exclude site dependencies/builds, planning records and exploratory assets.
Reader guides currently reference no local image or syntax assets.

Extract the archive into a new directory outside the checkout and run from its
`zirql-0.1.0` directory:

```sh
cargo run --locked --example words
cargo run --locked --example csv -- 'let selected = [p for p in rows if p.active == "yes"]; selected | count' examples/csv/people.csv
cargo run --locked --example csv -- 'let selected = [p for p in people if p.active == "yes"]; [{"team": t.label, "count": length([p for p in selected if p.team == t.id])} for t in teams]' people=examples/csv/people.csv teams=examples/csv/teams.csv
```

Expected results are the three adjacent word pairs, `3`, and team counts
`1, 2, 0` in fixture order. Check a missing CSV column fails with a query location
and empty stdout. Build a small external Rust consumer against the extracted
crate using the public reader/evaluator, checked host validation and native
program/function APIs required by the adapters.

For a local combined release check, Cargo 1.99's native multi-package packaging
can verify the selected Zirql, Zincite and Zirium candidates together while
retaining their registry version requirements. This is prepublication artifact
evidence and does not change Zirql's Rust 1.88 minimum.

## Check the documentation site

From `site/`, use Node.js 22.12 or later:

```sh
npm ci
npm run check
SITE_URL=https://zayenz.se SITE_BASE=/zirql/ npm run build
npm run preview -- --host 127.0.0.1
```

Review `/zirql/`, setup, CSV and the language/API references. Check local links
and fragments, assets, canonical paths, light/dark themes, narrow navigation and
search. The output is `site/dist/`. Building does not deploy it. Follow the
repository's `site/PUBLISHING.md` for the separate reviewed personal-site import.

## Publish after authorization

Publish Zirql before either consumer requires it from crates.io:

```sh
cargo publish --locked
```

Wait for the registry index and archive to expose 0.1.0. In a new external Rust
project, use an exact registry requirement `zirql = "=0.1.0"`, with no local/Git
patch, and repeat the public API control. Compare the downloaded registry archive
with the reviewed archive, and check the versioned docs.rs API.

Only then verify consumer registry packages. Zincite's order is syntax, lint,
fmt/query, then the root package. Zirium 0.5.0 also depends on Zirql 0.1.0.
Consumer publication and artifacts follow their own procedures. Retain the
yanked reservation and earlier usable prereleases unchanged.

Create `v0.1.0` at the reviewed source commit and push it only with authorization.
Use `CHANGELOG.md` as the release notes. Import and deploy the reviewed site
separately, then check the live installation guides and links. Registry upload,
tag pushes and site deployment are separate actions; successful local checks
do not establish any of them.
