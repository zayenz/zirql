+++
schema_version = 1
id = "base-084"
key = "release-candidate-acceptance"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-082", "base-083"]
+++
# Verify the three release candidates and prepare the publication handoff

## Outcome

The exact three candidate releases have independently checked source packages, runnable public examples, installable local artifacts, coherent sites and a concrete ordered external-publication handoff.

## Context

Prerequisites prepare versions and all source/docs/CI. Native Cargo+1.99.0 can package unpublished dependent members with its temporary registry; Root verified a disposable two-crate example at /private/tmp/zirql-release-20261016-workspace-package-probe.json. Use a simple disposable combined workspace over copies of the exact candidate sources to verify normalized registry dependency archives before public upload, not a custom registry or release framework. Real public registry availability and final unpatched registry installation remain distinct external checks. Read the accepted per-project candidate results and release instructions. Language discovery supplies current bounded cross-host probes to replay against candidates.

## Boundaries

- No optional features, platform expansion, custom registry server, generalized release orchestrator, benchmark campaign or growing test matrix. No new product tests for artifact bookkeeping.
- Distinguish actual archive/installed binaries from cached checkout code, and staged artifact/source checks from public registry installs. Never invent Git revisions or substitute local paths while claiming registry resolution.
- No publication, pushed tags, release assets or live site import/deployment before Root receives any still-needed external authorization. Preserve old usable prereleases and yanked name reservation.

## Done when

- [ ] All seven Rust candidate packages (Zirql, five Zincite crates, Zirium) have reviewed normalized dependency contracts, useful contents and local package verification with exact version/source/archive hashes. The two documented Zirql examples run from the archive.
- [ ] A few ordinary cross-host/CSV report/function/empty/refusal/buffering checks succeed; the documented first-use formatter/linter/query and MLIR/Python examples run on candidate artifacts or clean installs. Keep CSV function declaration limits explicit.
- [ ] Attempt required configured CI and existing supported artifact jobs at exact candidate commits. Record successful hosted proof or identify the precise unavailable external prerequisites for the attended publication gate. Local wheel/sdist/CLI archive checks, version agreement and three site build/link/desktop/mobile/search proof are complete.
- [ ] A concise reviewable release handoff identifies exact commits, artifacts/checksums, dependency publication order, supported platforms, pending real-registry install checks and the exact external actions requiring authorization. Root retains those gates without claiming the releases published.

## Validation

- Use native Cargo 1.99 multi-package verification and inspect/exercise extracted archives; verify unchanged code content if temporary workspace metadata is adapted. Don’t retain probe-only scaffolding as product code.
- Reuse passing prerequisite full suites where source is unchanged; rerun only materially changed checks and a few actual end-to-end candidate examples.
- Root reviews all three site installation/start/reference pages at desktop/mobile, navigation/search/themes/assets/links, and independently accepts the complete candidate handoff.
