+++
schema_version = 1
id = "base-034"
key = "zirium-functions"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-033", "base-031"]
+++
# Expose typed recursive functions in Zirium programs

## Outcome

Zirium users can define and call typed native query functions with guaranteed compatible tail calls through inline, file and library entry points.

## Context

Read the common fn/signature contract, Zirium AGENTS.md and current query program/parser/builder/projection code. Adapt the brief’s subtree_cost example to real native operations and type names; direct children mean containment, not SSA users. Use the shared runtime exclusively.

## Boundaries

- Zirium integration only, plus necessary shared adapter seams; no new edit/diff transaction contract or package publication.

## Done when

- [x] Common fn declarations, types, lexical locals and calls lower from Zirium’s existing expression/native-query forms to the shared representation, including direct/mutual and tail recursion.
- [x] Host-declared Operation/SSA and relevant result contracts preserve native identity and document ownership, with located wrong-argument/result errors.
- [x] The subtree-cost example produces 8 for an unannotated root and cost-3/cost-4 leaves; malformed cost data fails. A tail accumulator crosses the active-depth ceiling under sufficient work.
- [x] Short existing queries need no function preamble, existing preflight/purity/emission behavior remains compatible, and guides include complete executable examples.
- [x] Record separate consumer/core commits and update the pinned Git dependency.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse a native tree example and scalar tail example for frontend acceptance, runtime type refusal and budget exhaustion; avoid a full syntax matrix.

## Result

Expose typed recursive native query functions through Zirium inline, file and Rust builder entry points, preserving native identity, preflight and active call limits. Zirium commit 9778d00957d4451190cebd2d1f9a6d4994302c8f pins shared source cdb64efb4b9685ee2f494b7c3f04fc26e2f3527a; its preceding task-owned core transports are cbcc681, d809232 and 784e7e9.

Validation:

- Core stable and Rust 1.88 workspace tests pass: 60 public controls and six doctests on each toolchain; format and warning-fatal all-target Clippy pass.
- Zirium stable and Rust 1.88 all-workspace/all-target/all-feature locked serial tests pass: 627 passed and five existing ignored on each; Clippy, format, three doctests and warning-fatal rustdoc pass.
- Fresh private CPython 3.11–3.14 wheels and actual imported extension bytes agree; Ruff, explicit-interpreter ty and pytest pass on each, with 171 passed and two existing macOS /dev/full skips. All 1825 canonical Python files and symlinks remain unchanged.
- Executable inline, saved-file and direct typed Rust examples return subtree cost 8 and accumulator 500500, including tail execution at active depth 1. Native identity/ownership, type refusals, lexical arguments, all-written preflight, path-target active-call/lazy behavior and later-error buffering controls pass.
- Independent whole review found one P2; Root independently reproduced it, the same author completed one correction, and a fresh focused reviewer returned PASS at W05fb28fa740eb665. Exact core and raw consumer comparisons were equal before acceptance.
- Sequential site checks/builds and actual desktop/mobile guide, search and navigation inspection pass. Four core and three host guide inputs and all 55/66 reviewed outputs remain byte-identical through correction.
- Twenty-seven whole-process native-tree, tail and retained-alias projection samples remain attributed to their original pre-correction source/pin 784e7e9. They include setup/rendering and establish no allocator-only or speedup claim; corrected CLI probes preserve the required outputs.
- Core source CI 38601104349 passes both jobs; Zirium Quality 38603942867 passes all 12 Rust, Linux/macOS CPython-wheel and source-distribution jobs for consumer commit 9778d00957d4451190cebd2d1f9a6d4994302c8f.
- git diff --check and zdev check base pass; unrelated assets and the original clean Zincite checkout are preserved. No package was published.
