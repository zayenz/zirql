+++
schema_version = 1
id = "base-038"
key = "computation-acceptance"
area = "base"
status = "open"
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

- [ ] Compare small-query, repeated-projection and large-binding baseline outputs/statuses and phase time/memory, recording revisions, samples/spread and any materially regressed path.
- [ ] Measure wide reports, bounded deep/non-tail recursion, long tail loops, repeated immutable arguments and stable sorting; distinguish work/depth refusals from completed runs.
- [ ] Temporary evidence shows no body re-preparation or deep copying of unchanged function arguments, no frame/history growth proportional to tail-call count, and prompt release of dead intermediates.
- [ ] Guides state runtime type boundaries, tail versus non-tail limits, optional arithmetic/defaults, stable ordering, string semantics and context-limited preparation reuse, with executable examples for both hosts.
- [ ] Acceptance reports numerical results and unresolved limitations honestly; any remaining substantive regression becomes a focused follow-up, not a relaxed budget or unsupported efficiency claim.

## Validation

- Use named paired measurements and output/error comparisons. Run existing relevant host/core suites after final changes; documentation-only changes use record/docs checks. No new timing tests.
- Build local documentation when guides change; publication/deployment remains separate.
