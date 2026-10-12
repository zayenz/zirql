+++
schema_version = 1
id = "base-007"
key = "keyed-comprehension-values"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-005"]
+++
# Construct enum-aware keyed and multidimensional Zincite reports

## Outcome

Shared construction preserves key identities and axes while Zincite validates MiniZinc-specific enum and rectangular shape rules.

## Context

Read background/comprehensions.md and the keyed, set, empty-axis, numeric and report sections of Zincite's contract. Read structured.rs literal shapes and the live public model type/enum facts. Extend the shared value representation only where these report consumers need it; source/model resolution and MiniZinc shape policy remain host operations.

## Boundaries

- Keep positional, keyed, multidimensional and nested collections distinct; no inferred enum identity from source spelling.
- No data edits in this task. Generic maps do not inherit MiniZinc rectangular-array restrictions.
- Implement Zincite's supported homogeneous fixed sets and explicit nominal-member resolution without a core MiniZinc dependency.

## Done when

- [ ] Keyed and multidimensional comprehensions retain typed keys, rank, established axis order and row-major iteration in native results.
- [ ] Zincite resolves enum-member identity/order through its public facts and enforces contiguous/rectangular axes and the documented empty-axis policy.
- [ ] Duplicate or incompatible keys, missing cells, uncertain identity and unsupported string sets receive located refusals rather than key compaction.
- [ ] Tuple, record, typed-set and keyed-array output use the existing proposed Zincite tagged report shapes with completeness preserved.
- [ ] Key/source helpers and shape checks consume the request budget and do not evaluate arbitrary model expressions.

## Validation

- Use one compact explicit integer/enum multidimensional report plus missing-cell, duplicate-key and unestablished-empty-axis refusals; reuse current literal controls where sufficient.
- Run Zirql and Zincite required checks, including changed-record checks.
