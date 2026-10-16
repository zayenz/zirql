+++
schema_version = 1
id = "base-082"
key = "zincite-010-release-candidate"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-079", "base-081"]
+++
# Prepare the Zincite 0.1.0 package graph and reader documentation

## Outcome

Zincite 0.1.0 has publishable sibling dependency contracts, a bounded CI gate, useful package contents and accurate install/usage/release documentation.

## Context

The current root/query Cargo package commands fail because sibling path edges omit versions; zincite-query also omits the registry version on its reviewed Git Zirql dependency. All five Zincite crates currently inherit 0.1.0 but registry packages are only name reservations and zincite-query has no published entry. No hosted CI is tracked. Read workspace/all crate manifests, build.rs/CLI version output, lockfile, current installer/support/reference guides and the native query contract. Consume the independently reviewed Zirql 0.1.0 source from its prerequisite. Pending documented formatter work is owned by the other prerequisite, not an optional omission.

## Boundaries

- Give all sibling and Zirql release edges their actual 0.1.0 version requirements, with local workspace paths for sibling development. Until registry publication, a real reviewed Zirql 0.1.0 Git+version dependency may keep source builds usable; its normalized crate dependency must target registry 0.1.0. Final publication acceptance checks actual registry resolution.
- Keep existing root and standalone commands and MiniZinc/source/edit/completeness contracts. First distribution may use verified Cargo installation; new binary platforms or a generalized publisher are optional and deferred.
- Keep packages focused on needed sources, licenses, metadata and runnable referenced material. Add one small CI workflow matching required Rust checks, including relevant package readiness without hiding a failed release gate.
- No registry upload, release tag push or live deployment. Preserve unrelated notes/assets; do not change old warning policy solely to expand the test matrix.

## Done when

- [ ] All five normalized manifests expose coherent 0.1.0 registry edges and package metadata/contents. Lockfiles and CLI/library versions agree; selected MSRV/support claims are demonstrated.
- [ ] Current source remains buildable against the actual reviewed core revision. Native Cargo multi-package verification can check unpublished sibling archives; any remaining actual-registry availability gate is explicit and reserved for final release acceptance.
- [ ] The minimal CI configuration runs required checks. Demonstrate its commands locally; hosted execution at the candidate commit belongs to final candidate acceptance when available. Release notes and installation/support guides describe 0.1.0 commands, formatter/linter/query behavior and supported limitations, with runnable ordinary examples.
- [ ] Required affected Rust/library/CLI/docs/site/link/UI checks pass. Root independently verifies and records separate consumer/framework commits and candidate artifacts.

## Validation

- Run focused format/lint/query and later-error controls plus required current-stable workspace fmt/fatal Clippy/tests and warning-fatal docs. Retain the existing current-stable support claim; do not introduce a Zincite MSRV or release-only compiler matrix.
- Inspect every normalized package manifest/list and workspace Cargo dry-run semantics. No fake registry version, path-patched install represented as registry proof or new prerelease.
- Validate CI commands/configuration locally, all relevant site check/build/links and Root desktop/mobile install/guide checks; documentation needs no new tests. Attempt hosted candidate CI during final acceptance; unavailable credentials or permissions remain an explicit late external gate.
