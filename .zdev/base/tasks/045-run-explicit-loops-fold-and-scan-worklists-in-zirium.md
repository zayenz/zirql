+++
schema_version = 1
id = "base-045"
key = "zirium-worklists"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-040", "base-041", "base-042", "base-043"]
+++
# Run explicit loops, fold and scan worklists in Zirium

## Outcome

Zirium users can execute explicit-state loops, named fold/scan steps and native worklist queries using the shared runtime.

## Context

Read the selected loop/library contracts and worked example. Build on zirium-patterns and existing host functions; adapt adjacency to Operation identity and direct SSA-user edges. The host must expose complete adjacency or a located refusal. Use an acyclic SSA-user case and a generic shared cyclic fixture (do not fabricate invalid MLIR cycles).

## Boundaries

- Zirium integration plus necessary shared adapter seams; no solver execution, new rewrite/diff effects, all-path search, package publication or website deployment.

## Done when

- [x] Both text and typed-library entry points expose loop/break/continue, named fold/scan, typed queue/identity-set constructors and operations through the shared evaluator.
- [x] The host supplies explicit native identity and complete-or-error adjacency views; identity-set membership and target comparison agree. Unknown/incomplete facts cannot become false reachability.
- [x] The reachability program handles target-is-start, reachable and disconnected targets, duplicate routes and cyclic work where the host can represent it. Pattern decomposition of optional pop results does not copy or rescan the remaining queue. The reducer example uses a destructured step parameter and retains scan states correctly.
- [x] Pure loop scopes, runtime state/step checks, tail calls, locations, buffering and default limits remain compatible. Invalid direct rendering of queue/set values is a located refusal.
- [x] Document complete executable programs and measure small-query overhead plus increasing worklists, aliases and scan states with exact revisions, samples/spread and available peak memory; separate limit failures from throughput and report any unresolved scaling concern.
- [x] Record separately reviewable consumer/core commits and pinned Git dependencies.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Run Zirql and the affected host’s required checks. Reuse compact examples for inline/file/typed-library agreement, located failures and later-error output buffering; record separate consumer commits and Git pins.
- Reuse shared controls and real host fixtures; add focused host acceptance for adjacency/identity/unknown facts and transfers. Use temporary allocation/copy evidence, not timing assertions.

## Result

Zirium text and typed APIs run shared explicit loops, fold/scan and persistent native worklists with complete SSA-user adjacency, exact owner/revision identity, finite budgets and buffered rendering refusals. Core and consumer changes are separately attributable; dependency pinned to ea30035.

Validation:

- Independent whole-task review1/correction0 PASS; immutable W1a843d03d46e8b20 comparisons equal and full consumer index guard unchanged.
- Fresh core fmt, strict Clippy and 111 tests; fresh Zirium stable fmt, all-feature strict Clippy, 640 tests (5 ignored), serial tests, 3 doctests and warning-denied rustdoc pass. Retained source-matched Rust 1.88 and private CPython 3.11–3.14 matrices pass (175 tests/2 skips each).
- Matched final-v2 167-process measurements, 334 stream hashes and 33 statistical rows checked; aliases and scan scaling, result drops and refusal limits documented. Both sites check/build pass; desktop/mobile, search, keyboard and local links verified.
