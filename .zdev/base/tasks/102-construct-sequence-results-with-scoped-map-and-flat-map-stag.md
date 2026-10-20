+++
schema_version = 1
id = "base-102"
key = "constructing-pipelines"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Construct sequence results with scoped map and flat_map stages

## Outcome

Short pipelines transform each sequence item into a value or a sequence using shared expression scopes.

## Context

Current constructed pipelines filter and order values; comprehensions construct results. Predicate arguments already provide named patterns, lexical captures and _. Start at src/reader.rs, src/value.rs, src/value/execution.rs and scoped-predicate tests; docs/language-reference.md and docs/csv.md show the current surface. Use a capacity report relevant to Zincite and a CSV row example. Read the brief section Everyday language exploration (2026-10-20) for scope, CP alignment and documentation obligations.

## Boundaries

- No tap/inspection effects, lazy iterators, implicit iteration over dictionaries/keyed arrays, or general sorting/reducer callback expansion.
- Keep this slice in Zirql and its local examples; record concrete consumer adoption requirements without changing consumer repositories.
- No Gecode dependency, symbolic CP lowering, first-class function system, publication or deployment.

## Done when

- [ ] Shared textual and callable map(pattern => expression), map(expression using _), and flat_map equivalents accept positional sequences and return eager positional sequences; flat_map concatenates one level of positional sequence results in input order.
- [ ] Nested arguments rebind _ while named outer captures remain lexical; existing strict pattern mismatch, purity, preflight and cumulative budget rules apply. Reject non-sequence flat_map results without partial output.
- [ ] A focused report demonstrates lexical capture, order and retained native values through the existing host harness; cover empty input, a meaningful wrong-result-kind refusal and expansion exhaustion without a syntax matrix.
- [ ] Update the relevant public language guide and callable/embedding documentation with the implemented contract, supported host/example entry point, limitations and a runnable example with expected output. Document necessary consumer adoption separately.

## Validation

- Use the focused behaviour checks described in Done when; run the documented local example and compare its result.
- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace
- git diff --check
- zdev check base --format json
