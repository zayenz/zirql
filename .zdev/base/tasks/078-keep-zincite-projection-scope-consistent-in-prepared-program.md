+++
schema_version = 1
id = "base-078"
key = "zincite-shared-prepared-projections"
area = "base"
status = "open"
complexity = "standard"
afk = true
blocked_by = ["base-077"]
+++
# Keep Zincite projection scope consistent in prepared programs

## Outcome

Zincite uses the shared projection runner in prepared values and functions, with the same singleton-relative field scope and empty-input scalar admission as ordinary projections.

## Context

Root rebuilt the current canonical Zincite CLI and reproduced a concrete inconsistency on two model items, int: a=1; int: b=2;. report items | project(d,{"n":count}); returns1 in each row, while adding fn n()->int{1} makes each row return2. Also items | head(0) | project(d,{"x":d}) rejects its native field normally, but adding that unrelated declaration succeeds with an empty report. In crates/zincite-query/src/program/composition.rs, NativeReader::projection lowers prepared/function projections to an ordinary native In comprehension with a Record head. That lowering does not preserve projection scalar admission or singleton-relative head input. Base-077 supplies the projection-specific active NativeArguments entry point and shared scoped field preparation. Read that API, CompositionHost argument discovery/dispatch and function Session/preparation/wrapping before adapting the host. This is a concrete shared-contract fix, not a symmetry-only dependency update.

## Boundaries

- Use existing shared projection and prepared argument execution, retaining one core row/field evaluator. Replace the semantics-changing projection-to-comprehension lowering with an appropriate existing native-stage/value route; no host subject/field loop, parser rewrite, new projection representation or generic protocol.
- Preserve MiniZinc native selections/owners/source coordinates, unknown/completeness handling, explicit scalar conversion, typed output envelope, written row/field order and duplicates, lexical no-shadowing, original versus singleton-relative input, finite cumulative budgets and later-error output buffering. All written fields are admitted before subject enumeration, including empty input.
- Change the affected Zincite integration, actual pinned shared Git dependency and directly affected current-behavior guide only. Keep MiniZinc model evaluation and data-edit rules in the host; no broader edit policy, registry publication, deployment, unrelated formatting or framework redesign.

## Done when

- [ ] The exact two-item report returns n=1 per row both with and without an unrelated function declaration through actual library and inline/file CLI paths. A saved original input remains available separately and counts2.
- [ ] The native-valued field on empty input is rejected before output in both modes. Valid empty projected reports and existing function/nested projection reports retain their supported schemas and results through shared execution.
- [ ] Compact existing public controls cover the two regressions plus any otherwise-uncovered active function/located late-failure boundary. Reuse existing order/duplicate/native-owner/completeness/budget controls; no generic syntax matrix, callback-count or timing tests.
- [ ] The actual Zincite manifest, lock and locked metadata resolve the independently accepted base-077 shared Git revision. Required affected-host Rust/library/CLI checks pass, existing model/data-edit contracts remain correct, and any directly affected guide explains current behavior.

## Validation

- Extend focused existing query/function/projection library coverage and a compact inline/file CLI control for the concrete count and empty-field failures; run rebuilt actual Zincite command paths and later-error empty stdout checks. Inspect native-stage scoped argument discovery and active dispatch rather than masking the defect with parser heuristics.
- Run git diff --check, cargo fmt --all -- --check, default and explicit Rust1.99 warning-fatal workspace/all-target Clippy and locked workspace tests, plus the affected host required Rust1.88/docs checks. Use existing supported feature/check commands; do not invent a new matrix. Verify actual locked Cargo metadata against the reviewed pushed Git SHA, without path overrides or package releases.
- Root runs applicable zdev checks and independent whole-task verification before separate attributable consumer/framework-record commits. Existing site check/build/link and desktop/mobile review apply only to edited reader pages. Preserve all unrelated Zincite formatter, site and record edits.
