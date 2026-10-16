+++
schema_version = 1
id = "base-081"
key = "zirql-010-release-candidate"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-080"]
+++
# Prepare the self-contained Zirql 0.1.0 release candidate

## Outcome

Zirql 0.1.0 has a small self-contained crate, accurate public API/language documentation and reviewable release notes/artifacts.

## Context

The checked packaging audit found unanchored Cargo include patterns collect ignored site/node_modules README files and omit words/csv examples and reader docs. Root independently reproduced 263 paths including 228 site entries. Current manifest is alpha.5 and README/setup falsely describe APIs now implemented as Git-only, including contradictory worklist support. Read Cargo.toml, README/src/lib.rs public exports, examples, getting-started/host-integration/native-programs and current API/function guides. Language discovery passed bounded ordinary programs in both consumers and CSV; it does not replace final release acceptance. Audit results are /private/tmp/zirql-release-20261016-language-audit.json and packaging-audit.json.

## Boundaries

- Keep one library crate and current supported API/language contracts. Correct documented contradictions and concrete public usability failures; no optional language extensions or broad API redesign.
- Use root-anchored package selection. Include the runnable word/CSV examples, required fixtures, licenses and referenced reader material; exclude site dependencies/build outputs, .zdev, private notes and untracked exploratory assets.
- Prepare local artifacts and release instructions only. No registry upload, tag push or deployment. Root may push a reviewed real source revision needed by dependent candidate work; no extra prerelease.

## Done when

- [x] Cargo/lock and public release references consistently identify 0.1.0. A concise changelog states the supported shared language, API and limits; earlier release history is retained honestly.
- [x] Package lists are stable with or without installed site dependencies. The archive builds and its word/implicit-CSV/named-CSV examples run from an extracted directory outside the repository, with relative guide/fixture links intact.
- [x] A small standalone public-API consumer demonstrates the APIs the two hosts need. Documentation uses actual 0.1.0 registry syntax and versioned checkout examples while retaining explicit CSV/program and host support limits.
- [x] Required Rust checks and affected site check/build/link/UI pass. Root independently verifies, commits and retains exact candidate archive hashes and a real reviewed 0.1.0 source revision for consumers.

## Validation

- Run actual cargo package verification and inspect archive contents; a few public API/example/refusal controls suffice. No product tests for metadata/docs.
- Run required current-stable formatting, fatal Clippy/workspace tests, existing MSRV 1.88 tests, doctests/warning-fatal docs and relevant site checks. Reuse unchanged language discovery evidence alongside candidate checks.
- Distinguish local artifact checks from published-registry availability. Root handles external publication only after final review.

## Result

Prepared Zirql 0.1.0 with a self-contained 60-file crate, runnable word/CSV examples, current API/language/setup documentation and release notes/procedure; runtime and public API unchanged.

Validation:

- Independent PASS: stable fmt/fatal Clippy/tests, Rust1.88 tests, fatal docs/doctests; package verification and file selection independent of site dependencies; extracted word/single/named CSV plus refusals; external artifact API caller; site build, 1,215 local targets and desktop/mobile navigation/search. Guard and source/index byte comparisons exact; clean commit repack retained by Root.
