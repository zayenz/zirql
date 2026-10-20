+++
schema_version = 1
id = "base-103"
key = "selective-generators"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Select matching values with explicit case generators

## Outcome

Comprehensions can skip pattern mismatches explicitly while ordinary bindings remain strict.

## Context

List/record/optional patterns already exist; generator mismatches currently fail. Reuse src/value/pattern.rs and shared comprehension lowering rather than a second matcher. Read docs/functions-and-control-flow.md, docs/values.md and background/comprehensions.md. Mixed report records and present optional measurements motivate the feature. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- No new native syntax-template engine or alteration of existing pattern equality and absence rules.
- Keep this slice in Zirql and its local examples; record concrete consumer adoption requirements without changing consumer repositories.
- No Gecode dependency, symbolic CP lowering, first-class function system, publication or deployment.

## Done when

- [ ] The shared reader and callable representation support an explicit case modifier on a generator in both comprehension spellings; only a structural, literal or presence pattern mismatch skips that candidate.
- [ ] Ordinary generators and let bindings retain strict mismatch errors. Successful match bindings are local to that candidate; later sources, guards and heads retain current scope/order. Errors and unknown guards are not treated as mismatches.
- [ ] One compact record/optional selection example and focused strict-versus-selective and runtime-error controls establish behaviour, including empty results and budgeted rejected candidates.
- [ ] Update the relevant public language guide and callable/embedding documentation with the implemented contract, supported host/example entry point, limitations and a runnable example with expected output. Document necessary consumer adoption separately.

## Validation

- Use the focused behaviour checks described in Done when; run the documented local example and compare its result.
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- git diff --check
- zdev check base --format json
