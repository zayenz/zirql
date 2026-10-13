+++
schema_version = 1
id = "base-028"
key = "core-ci-msrv"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Run core validation and the declared Rust version in CI

## Outcome

Zirql pushes and pull requests run the existing core quality checks and verify the declared minimum Rust version.

## Context

The reviewed core revision 6fef2436a311b6f83379d129d3000db0b494b314 has no repository CI configuration. Cargo.toml declares rust-version 1.88 and the brief requires formatting, Clippy and tests. These checks passed locally in this review, but the repository does not continuously enforce them. This is a maintenance improvement; no minimum-version failure was observed.

## Boundaries

- Use the existing single-crate checks and committed Cargo.lock; add no product tests or release/publishing automation.
- Keep site deployment and both consumer repositories outside this task.

## Done when

- [ ] Pushes to main and pull requests execute formatting, Clippy with the repository warning policy, and workspace tests including doctests.
- [ ] A Rust 1.88 job exercises the locked crate and test suite; stable-toolchain lint/format checks have an explicit owner.
- [ ] The workflow completes successfully on the resulting revision and its required commands are documented without duplicating the full contributor guidance.

## Validation

- Run the exact workflow commands locally using installed supported toolchains; do not change the crate edition or declared minimum version merely to pass.
- Observe one successful GitHub Actions run for the resulting revision.
- Run git diff --check and zdev check base --format json.
