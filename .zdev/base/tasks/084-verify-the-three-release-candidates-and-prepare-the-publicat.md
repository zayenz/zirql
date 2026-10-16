+++
schema_version = 1
id = "base-084"
key = "release-candidate-acceptance"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-082", "base-083", "base-087", "base-088", "base-089", "base-096"]
+++
# Verify the three release candidates and prepare the publication handoff

## Outcome

The exact three candidate releases have independently checked source packages, runnable public examples, installable local artifacts, coherent sites and a concrete ordered external-publication handoff.

## Context

Prerequisites prepare versions and all source/docs/CI. The scoped-predicate integration and hosted Linux CI correction are required release blockers; use their refreshed core and consumer commits and supersede all pre-filter candidate artifacts. Native Cargo+1.99.0 can package unpublished dependent members with its temporary registry; Root verified a disposable two-crate example at /private/tmp/zirql-release-20261016-workspace-package-probe.json. Use a simple disposable combined workspace over copies of the exact candidate sources to verify normalized registry dependency archives before public upload, not a custom registry or release framework. Real public registry availability and final unpatched registry installation remain distinct external checks. Read the accepted per-project candidate results and release instructions. Use existing passing core and host tests as language evidence; language-discovery probes are optional diagnostics for concrete uncovered behavior, not another release gate.

## Boundaries

- No optional features, platform expansion, custom registry server, generalized release orchestrator, benchmark campaign or growing test matrix. No new product tests for artifact bookkeeping.
- Distinguish actual archive/installed binaries from cached checkout code, and staged artifact/source checks from public registry installs. Never invent Git revisions or substitute local paths while claiming registry resolution.
- No publication, pushed tags, release assets or live site import/deployment before Root receives any still-needed external authorization. Preserve old usable prereleases and yanked name reservation.

## Done when

- [x] All seven Rust candidate packages (Zirql, five Zincite crates, Zirium) have reviewed normalized dependency contracts, useful contents and local package verification with exact version/source/archive hashes. The two documented Zirql examples run from the archive.
- [x] Zincite and Zirium build against the same reviewed Zirql Git revision with their relevant existing tests passing, and the CSV example runs. This is sufficient shared-language integration evidence; no separate cross-consumer replay campaign is required. The documented first-use formatter/linter/query and MLIR/Python examples run on candidate artifacts or clean installs. Keep CSV function declaration limits explicit.
- [x] Attempt required configured CI and existing supported artifact jobs at exact candidate commits. Record successful hosted proof or identify the precise unavailable external prerequisites for the attended publication gate. Local wheel/sdist/CLI archive checks, version agreement and three site build/link/desktop/mobile/search proof are complete.
- [x] A concise reviewable release handoff identifies exact commits, artifacts/checksums, dependency publication order, supported platforms, pending real-registry install checks and the exact external actions requiring authorization. Root retains those gates without claiming the releases published.

## Validation

- Use native Cargo 1.99 multi-package verification and inspect/exercise extracted archives; verify unchanged code content if temporary workspace metadata is adapted. Don’t retain probe-only scaffolding as product code.
- Reuse passing prerequisite full suites where source is unchanged; rerun only materially changed checks and a few actual end-to-end candidate examples.
- Do not duplicate Zirql language coverage in each consumer or replay cross-host report/function/empty/refusal/buffering cases already covered by passing suites. Add a focused integration check only for a concrete adapter defect or uncovered behavior. Keep package, clean-install, CI and documentation checks.
- Root reviews all three site installation/start/reference pages at desktop/mobile, navigation/search/themes/assets/links, and independently accepts the complete candidate handoff.

## Result

Prepared independently verified Zirql 0.1.0, Zincite 0.1.0 and Zirium 0.5.0 candidate artifacts and all three sites. Portable handoff, inventory and checksums are retained in target/release-candidates/2026-10-16/. Registry-only finalization, publication and live deployment remain the attended base-085 gate.

Validation:

- Independent whole review passed: exact source/product bytes, 447 bundle checksums, seven normalized Rust packages plus clean core archive, wheel/sdist contents and dependency contracts.
- Independent extracted Words/CSV, installed Zincite formatter/linter/query, strict Zirium CLI and isolated Python wheel with semantic SSA checks passed; retained accepted full suites and affected integration reviews apply.
- Exact candidate Quality/CI and supported nonpublishing artifact jobs passed; all three publication jobs were skipped. All 59 site pages and 3,737 local link/asset/fragment checks passed, with matching accepted desktop/mobile/navigation/search/theme evidence.
- Native snapshot W8e0e048c50ffdda1, all three repository guards and delivered artifact hashes matched before acceptance. No product source changes or registry publication occurred.
