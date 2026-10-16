+++
schema_version = 1
id = "base-083"
key = "zirium-050-release-candidate"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-081"]
+++
# Prepare Zirium 0.5.0 Rust and Python release candidates

## Outcome

Zirium 0.5.0 has consistent Rust/Python identities, actual Zirql 0.1.0 dependency contracts, checked artifact preparation and reader documentation.

## Context

Current Rust/extension/pyproject/uv metadata is 0.4.0 and changelog lacks 0.5.0. Current Cargo package normalizes its new Git dependency to registry alpha.5, which Root/audit confirms lacks required APIs and fails compilation. Read manifests, Cargo/uv locks, README/changelog, public Rust/Python interfaces and compatibility guide, quality/release workflows and site install/query/capability guides. Existing release matrix is Rust 1.88/stable, conventional CPython 3.11–3.14 Linux x86_64/macOS arm64 plus sdist, and Linux static-musl/macOS arm64 CLI archives. Keep this bounded support.

## Boundaries

- Set Rust library, non-publishable extension crate and Python distribution consistently to 0.5.0; consume reviewed Zirql 0.1.0 with registry version metadata. A real 0.1.0 Git+version edge may maintain prepublication source builds; normalized packages must resolve registry 0.1.0. Preserve internal extension publish=false and local sibling path.
- Preserve MLIR22.1/source/semantic/ownership/edit/query/buffering contracts and existing public support limits. No new Python/OS/ABI matrix or optional language feature.
- Prepare wheels/sdist and existing CLI archives with licenses/checksums and install checks. If adding a candidate CI route, it must produce artifacts without registry publication, tag creation or release/site publication; all existing publication jobs remain explicitly gated.
- No live registry uploads, release tag pushes or site deployment. Apply reader-first writing to release/install/API notes.

## Done when

- [x] Cargo manifests/lock, pyproject/uv metadata, CLI/Python versions, artifact filenames, release validation and 0.5.0 notes agree. Normalized Zirql dependency targets the actual selected 0.1.0 API.
- [x] A current local wheel installs into an isolated environment outside source trees, with correct import identity, stubs/py.typed/licenses and a meaningful public query. An sdist installs from its own contents with no stale local extension or undeclared checkout dependency.
- [x] Existing supported CLI archive preparation is reviewable, with locally available artifacts checked. Candidate/quality workflow configuration preserves the existing platform coverage without publishing. Hosted artifacts at exact candidate commits are checked during final acceptance when available; unavailable hosted prerequisites and actual registry availability remain explicit late gates.
- [x] Required Rust/Python/packaging and affected all-site install/support/reference examples, link and Root desktop/mobile checks pass. Root independently verifies and records separate commits and exact artifacts.

## Validation

- Run required current-stable Rust formatting/fatal Clippy/tests plus the existing MSRV 1.88 tests proportionate to affected packaging, doctests/warning-clean docs. Rebuild current extension and prove import identity before the full current Python suite and existing lint/type checks.
- Build/check local native wheel+sdist, clean installs and exact version agreement; validate existing hosted quality/platform/artifact configuration. Attempt hosted candidate artifacts during final acceptance and retain unavailable credentials or runner permissions as an explicit late external gate. Do not expand the matrix.
- Inspect tag/publication/candidate workflow conditions, actual normalized manifest/locked metadata and licenses/checksums. Run affected site builds/links and Root desktop/mobile checks.

## Result

Prepared consistent Zirium 0.5.0 Rust/Python metadata, candidate-only build workflow and reader/release guides. Independent whole review passed on all19 product paths; this is the pre-predicate packaging milestone, whose core pin and artifacts are superseded by required086087 integration before release acceptance.

Validation:

- Independent stable+Rust1.88 fatal all-target/all-feature checks, doctests/fatal docs; rebuilt CPython3.14 identity and175 tests/2 existing skips/Ruff/ty; fresh wheel/sdist isolated installs with empty sdist caches; normalized two-package nativeCargo1.99 archive verification; macOS11 CLI/license/checksum/strictStableHLO; workflow publisher gates; site build/links and Root desktop/mobile/search all pass. Real unpublished Zirql registry remains085 gate. SnapshotW0867827514051e53/all3guards unchanged, single completion comparison equal.
