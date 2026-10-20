# Release Zirql 0.1.3

## Prepare the source

Update the crate version, Cargo.lock, short changelog entry and current
installation, API and source links together. Keep historical measurements and
planning records at their recorded revisions.

Commit and push the candidate to `main`. Use the existing CI for formatting,
Clippy, stable tests and Rust 1.88 tests. Trust successful CI and the normal
publication command; release metadata does not reopen accepted code reviews.

## Publish the crate

After successful CI and authorized publication:

```sh
cargo publish --locked
```

Publish Zirql before consumers that require this version. Preserve the yanked
name reservation and earlier usable releases. Publish only the intended tag
and short release notes:

```sh
git tag -a v0.1.3 -m "Zirql 0.1.3"
git push origin v0.1.3
gh release create v0.1.3 --title "Zirql 0.1.3" --notes-file /path/to/release-notes.md
```

Do not use `git push --tags`. Report the version and public package/release URLs.

## Publish the guides

From `site/`, use Node.js 22.12 or later:

```sh
SITE_URL=https://zayenz.se SITE_BASE=/zirql/ npm run build
```

Follow [site/PUBLISHING.md](site/PUBLISHING.md) for the static import and existing
Git-based deployment at zayenz.se. The hosted build runs its normal release
checks; do not repeat those checks locally solely for an import. A successful
deployment completes the docs release. Further validation needs a concrete
failure, known defect or explicit request.
