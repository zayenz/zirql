+++
schema_version = 1
id = "base-038"
key = "computation-acceptance"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-034", "base-035", "base-036", "base-037"]
+++
# Measure and document the completed computation model

## Outcome

The implemented language and execution model have bounded, reproducible efficiency evidence and coherent host documentation.

## Context

Read the baseline and all new task results. Extend the existing small measurement drivers to the implemented recursive/native examples, repeated large arguments and stable ordering. Reconcile the brief’s proposals with actual accepted behavior without claiming an unmeasured speedup.

## Boundaries

- No corpus-wide benchmark system, arbitrary speedup promise, crate publication or personal-site deployment.

## Done when

- [x] Compare small-query, repeated-projection and large-binding baseline outputs/statuses and phase time/memory, recording revisions, samples/spread and any materially regressed path.
- [x] Measure wide reports, bounded deep/non-tail recursion, long tail loops, repeated immutable arguments and stable sorting; distinguish work/depth refusals from completed runs.
- [x] Temporary evidence shows no body re-preparation or deep copying of unchanged function arguments, no frame/history growth proportional to tail-call count, and prompt release of dead intermediates.
- [x] Guides state runtime type boundaries, tail versus non-tail limits, optional arithmetic/defaults, stable ordering, string semantics and context-limited preparation reuse, with executable examples for both hosts.
- [x] Acceptance reports numerical results and unresolved limitations honestly; any remaining substantive regression becomes a focused follow-up, not a relaxed budget or unsupported efficiency claim.

## Validation

- Use named paired measurements and output/error comparisons. Run existing relevant host/core suites after final changes; documentation-only changes use record/docs checks. No new timing tests.
- Build local documentation when guides change; publication/deployment remains separate.

## Result

Measured and documented the completed computation model; reconciled implemented brief status and published executable host guides. Separate consumer commits: Zincite 42f4d4be3e182ccc8196ed4ba63e318955d3555b; Zirium b4fca18a67f5170f7a1216139896a9ab5167f99e. Current and historical sources remain explicitly attributed; all observed timings include allocation-observer overhead. Paired small-report median 1.8248494 to 4.6310024 us (2.538x); eight-read binding 7.92923 to 9.35581 us (1.180x); projection1000 6465.854 to 3007.3965 us; binding256 131.22133 to 17.92668 us. These are instrumented fixture measurements, not production latency promises. Focused unresolved follow-up: investigate function-free preparation/continuation overhead using the exact small JSON report and eight reads of eight 128-byte strings; preserve one evaluator, native/owned result contracts, branch demand, locations and allowances. Future comparison must pair exact output/status, phase timing spreads and memory controls. No optimization, new task, budget relaxation or publication included.

Validation:

- Independent whole-task PASS with no findings/corrections; snapshot Wc78175083a3cc5b7 and fresh exact completion comparison equal. Verifier checked seven paths, all 1333 frozen files, 408 original raw records and all 68 five-batch groups. 68 warmups; 295 measured successes; 45 measured refusals; no target failures. Original twelve depth diagnoses/captures/durations preserved, with explicit derived public-error reclassification only. Private evidence /private/tmp/zirql038-author; verifier manifest /private/tmp/zirql038-whole-verifier/manifest.json.
- Bounded observer controls show one body preparation, unchanged argument sharing, bounded tail state and released intermediates. Retain qualifications for global allocator overhead, unavailable sort counters, historical attribution, RSS/fixture limits and unresolved regressions. CPython 3.11–3.14 target-free analysis, Ruff/ty and bounded runner resume/configuration/failure/timeout controls passed.
- All eight actual host inline/file/native/refusal replays match expected outputs; refusal stdout remains empty. Both hosts retain the existing pinned Git dependency. Documentation-only final changes: all three site checks/builds, cached/uncached diff checks and authoritative zdev check pass; no new product timing tests.
- Actual local documentation UI: four guides light/dark at desktop/mobile, bounded code/table overflow, navigation/menu/Escape/focus, all three searches, 98 local/cross-project links and anchors, canonical paths/assets/console passed; all 262 reviewed source/output hashes retained. Original failed zero-byte Zirium search output preserved; standalone rebuild and all final assets pass.
