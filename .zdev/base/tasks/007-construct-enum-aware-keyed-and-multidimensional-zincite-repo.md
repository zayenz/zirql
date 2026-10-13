+++
schema_version = 1
id = "base-007"
key = "keyed-comprehension-values"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-005", "base-016"]
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

- [x] Keyed and multidimensional comprehensions retain typed keys, rank, established axis order and row-major iteration in native results.
- [x] Zincite resolves enum-member identity/order through its public facts and enforces contiguous/rectangular axes and the documented empty-axis policy.
- [x] Duplicate or incompatible keys, missing cells, uncertain identity and unsupported string sets receive located refusals rather than key compaction.
- [x] Tuple, record, typed-set and keyed-array output use the existing proposed Zincite tagged report shapes with completeness preserved.
- [x] Key/source helpers and shape checks consume the request budget and do not evaluate arbitrary model expressions.

## Validation

- Use one compact explicit integer/enum multidimensional report plus missing-cell, duplicate-key and unestablished-empty-axis refusals; reuse current literal controls where sufficient.
- Run Zirql and Zincite required checks, including changed-record checks.

## Result

Zincite constructs enum-aware keyed and multidimensional reports through Zirql, preserving native key identity, ordered axes, shape policy, completeness and one request budget. Core source 9c3ce81; consumer source 4c9da0e and records 2efee89. Git development integration is pinned; later docs and measurement commits preserve product code.

Validation:

- Independent whole-task PASS at W851aa02328773f73; exact coordinator snapshot comparison passed before lifecycle mutation. All five completion conditions covered.
- Zirql and canonical Zincite fmt, workspace Clippy with denied warnings and workspace tests pass; Rust 1.88 query all-target checks, query tests and root CLI tests pass. Retained evidence applies to unchanged product source.
- All four installed commands verified; all 22 current installed query controls match complete stdout, stderr and statuses, including incomplete reports and buffered later errors.
- Public native/library controls preserve nominal identities, multidimensional axes, row-major cells and located shape/key refusals. Temporary reuse observations confirm retained parse and model facts; no permanent counters.
- git diff --check and zdev check base pass. Consumer canonical source/records are independently verified and pushed; distribution packaging is deferred under the approved pinned-Git development policy.
