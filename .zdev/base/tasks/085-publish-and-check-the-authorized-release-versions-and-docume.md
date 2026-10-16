+++
schema_version = 1
id = "base-085"
key = "publish-selected-first-releases"
area = "base"
status = "done"
complexity = "standard"
priority = "low"
blocked_by = ["base-084"]
+++
# Publish and check the authorized release versions and documentation

## Outcome

After explicit external-action authorization, selected releases are actually available, installable through their public registries/downloads and documented on the three deployed sites.

## Context

This is an attended external gate, not permission granted by task import. The prerequisite supplies exact reviewed commits/artifacts and publication order. Root asks only once the preparation is concrete; continue unrelated authorized preparation while waiting, but never upload or push release tags from an unanswered request. Root checks current external registry state before any retry. Zirql 0.1.0 must exist and resolve before consumer registry verification/publication. Zincite order is syntax, lint, fmt/query, root; Zirium 0.5.0 includes existing Rust/Python/binary channels. Site deployment follows the existing personal-site import route when authorized.

The user approved the selected registry, tag, asset and site publication actions on 2026-10-16. For final publication, use the existing package checks and CI and confirm actual publication/deployment status. The user explicitly ruled out additional representative clean installs; retain the completed installation evidence rather than repeating it.

## Boundaries

- Run only the external actions the user actually authorizes. Keep exact target versions/commits and independently reviewed artifacts; don’t replace published immutable packages, erase old usable versions or change reservations.
- Registry-version-only dependency/lock finalization may follow verified Zirql publication if needed; independently verify any resulting source/package delta before dependent uploads. No unreviewed source change rides a release tag.
- Use existing package and CI evidence without adding clean-install checks. Confirm actual registry/download versions, final registry dependency identity and deployed guide destinations. Don’t claim a prepared build was deployed.
- If external authorization or credentials are unavailable, preserve the review-ready state and report the exact remaining gate; do not complete this task or manufacture a release.

## Done when

- [x] The user has authorized the exact applicable external actions. Required hosted quality and existing supported artifact jobs are green at the exact candidate commits before dependent uploads. Zirql 0.1.0 resolves from crates.io and a clean exact-version consumer exercises required public APIs before downstream publication.
- [x] The five Zincite 0.1.0 crates and Zirium 0.5.0 Rust/Python/binary channels are published as authorized. Existing package checks and CI pass for the final registry-backed sources, and actual registry/download versions are confirmed. No additional clean installs are required.
- [x] Release tags/notes/assets and all three authorized deployed documentation sites agree on versions, install commands, support limits and links. Existing published versions remain intact.
- [x] Root independently verifies real external outcomes, records exact published artifact/checksum/commit evidence and completes only genuinely fulfilled gates.

## Validation

- Before uploads, compare exact candidate artifacts/manifests/source commits and current registry state. Follow the prepared dependency order and release procedures.
- Retain existing package/installation evidence and trust the configured CI and release pipeline. Confirm published versions and HTTPS deployment/link status; no additional clean installs, API exercises or test matrix.
- Root manages external actions, independent final verification, exact zdev records and separately attributable commits.

## Result

Published Zirql 0.1.0, all five Zincite 0.1.0 crates and Zirium 0.5.0 across crates.io, Python and CLI channels. Published the reviewed tags, notes and download assets, and deployed all three latest documentation sites through the personal-site production route. Final inventory and handoff are retained in target/releases/2026-10-16/.

Validation:

- Final independent whole-task review passed at Wd538264f38cb4a2a; registry checksums, source/tag identities, package contracts and release destinations agree. Required native and three-repository state comparisons passed.
- Exact-source CI and native package/dry-run checks passed. Zirium Release 38910565838 succeeded in all 18 jobs, including all three publishers; PyPI lists eight supported wheels and one sdist and GitHub lists both CLI archives and checksums.
- Production Netlify deployment 6ad22c6d825545afe164ac69 is ready/published at personal-site commit 4f4f76f951a87f3cf30e018dc9c88b589e2f71ca. Three exact latest imports and live homepage versions were confirmed; existing site and presentation evidence is retained.
- Applied the user instruction to trust existing package checks and CI without additional clean installs, API exercises or test replay. Existing prereleases/reservation and unrelated edits remain preserved.
