+++
schema_version = 1
id = "base-014"
key = "flat-native-set-sequencing"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-013"]
+++
# Preserve flat native set sequencing and publish the compatible runtime

## Outcome

The shared native program runner accepts Zirium's established flat left-associated set chains with one retained input/context/budget and unchanged recursive nesting limits, and supplies an independently verified usable source revision and fresh registry prerelease for consumer packaging.

## Context

Retain /private/tmp/zirql004-plan.json. Zirium's existing query_dsl flat_composition_does_not_consume_recursive_nesting builds 200 unions and passes baseline 5e6dfcb; the alpha.2 shared adapter fails at query_dsl.rs:265 because src/program.rs gives each Set node one logical depth and follows the left tree recursively in preflight and execution. Coordinator independently checked the failing control and released source. Consumer WIP is retained only in /Users/zayenz/projects/zirium and /private/tmp/zirql-base004-zincite, indexed by /private/tmp/zirql004-consumers-handoff.json; this child cannot edit those roots. Repair the core continuation/preflight directly, preserving native values and operand order rather than installing a host flat-set runner. Existing Group/selector nesting and finite work/result accounting still apply.

## Boundaries

- Core-only native program compatibility repair and necessary package/release documentation. No consumer edits, callbacks, error hierarchy change, comprehension expansion, data edits, legacy limit increases, or replacement of published alpha.2. Use a fresh independently verified prerelease under existing release authority; retain the yanked reservation.
- Initial task-owned path allocation: ["src/program.rs","src/eval.rs","tests/native_programs.rs","README.md","Cargo.toml","Cargo.lock"]. Coordination may extend it only after checking retained parent edits and sibling assignments.

## Done when

- [ ] A focused native borrowed-selection control runs at least 200 left-associated set operands without consuming recursive nesting solely for flat composition, preserving left-to-right operand observations and independent current input.
- [ ] Flat-set preflight and execution retain one context, immutable bindings and aggregate budget; actual grouped/nested selectors retain the supported nesting ceiling, resource refusals retain locations, and legacy host work accounting remains unchanged.
- [ ] Existing native program and pipeline controls, formatting, Clippy, tests including doctests, and clean locked package pass before independent verification and completion.
- [ ] The complete verified source revision and a fresh registry prerelease are published under approved authority; package/checksum and exact-version resolution expose the compatible program runner for consumer source/package pins. Alpha.2 is not overwritten and the reservation stays yanked.

## Validation

- Reuse tests/native_programs.rs for one focused borrowed-selection flat-chain control and existing nesting/budget/emission controls; no host test matrix or persistent counters.
- Run git diff --check;zdev check base --format json;cargo fmt --all -- --check;cargo clippy --workspace --all-targets -- -D warnings;cargo test --workspace;cargo package --locked. Independent verifier checks the complete child before completion/commit and approved fresh-prerelease publication.
- Verify registry package/checksum and exact-version native API resolution; after the child commit/publication, resume parent consumer implementation with its exact retained plan and external WIP attribution.
