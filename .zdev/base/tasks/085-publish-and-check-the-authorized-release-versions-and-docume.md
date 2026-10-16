+++
schema_version = 1
id = "base-085"
key = "publish-selected-first-releases"
area = "base"
status = "open"
complexity = "standard"
priority = "low"
blocked_by = ["base-084"]
+++
# Publish and check the authorized release versions and documentation

## Outcome

After explicit external-action authorization, selected releases are actually available, installable through their public registries/downloads and documented on the three deployed sites.

## Context

This is an attended external gate, not permission granted by task import. The prerequisite supplies exact reviewed commits/artifacts and publication order. Root asks only once the preparation is concrete; continue unrelated authorized preparation while waiting, but never upload or push release tags from an unanswered request. Root checks current external registry state before any retry. Zirql 0.1.0 must exist and resolve before consumer registry verification/publication. Zincite order is syntax, lint, fmt/query, root; Zirium 0.5.0 includes existing Rust/Python/binary channels. Site deployment follows the existing personal-site import route when authorized.

## Boundaries

- Run only the external actions the user actually authorizes. Keep exact target versions/commits and independently reviewed artifacts; don’t replace published immutable packages, erase old usable versions or change reservations.
- Registry-version-only dependency/lock finalization may follow verified Zirql publication if needed; independently verify any resulting source/package delta before dependent uploads. No unreviewed source change rides a release tag.
- Use actual clean registry/download installation without Git/path overrides, check versions and meaningful public commands, and verify actual deployed guides/links. Don’t claim a prepared build was deployed.
- If external authorization or credentials are unavailable, preserve the review-ready state and report the exact remaining gate; do not complete this task or manufacture a release.

## Done when

- [ ] The user has authorized the exact applicable external actions. Required hosted quality and existing supported artifact jobs are green at the exact candidate commits before dependent uploads. Zirql 0.1.0 resolves from crates.io and a clean exact-version consumer exercises required public APIs before downstream publication.
- [ ] The five Zincite 0.1.0 crates and Zirium 0.5.0 Rust/Python/binary channels are published as authorized. Independent exact-version registry/archive checks and isolated Cargo/Python/CLI installs pass without source overrides.
- [ ] Release tags/notes/assets and all three authorized deployed documentation sites agree on versions, install commands, support limits and links. Existing published versions remain intact.
- [ ] Root independently verifies real external outcomes, records exact published artifact/checksum/commit evidence and completes only genuinely fulfilled gates.

## Validation

- Before uploads, compare exact candidate artifacts/manifests/source commits and current registry state. Follow the prepared dependency order and release procedures.
- Run a few fresh clean registry installs/public first-use commands and HTTPS deployment/link checks. Distinguish unavailable platforms from verified hosted support; no new matrix.
- Root manages external actions, independent final verification, exact zdev records and separately attributable commits.
