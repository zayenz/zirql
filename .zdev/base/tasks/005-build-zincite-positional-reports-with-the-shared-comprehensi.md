+++
schema_version = 1
id = "base-005"
key = "minizinc-comprehension-reports"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-004", "base-015"]
+++
# Build Zincite positional reports with the shared comprehension evaluator

## Outcome

MiniZinc-style finite comprehensions construct Zincite reports using one Zirql evaluator for checked scalar expressions, scope and iteration.

## Context

Read background/comprehensions.md and Zincite docs/query-comprehension-contract.md in full. Start with positional arrays, records, tuples, finite scalar sources and ranges. Lower supported expression nodes from Zincite's existing CST to located Zirql expressions; the host source/query wrappers supply typed bindings. Shared operators belong in Zirql, while model facts and host-specific subset validation stay with Zincite.

## Boundaries

- Positional report construction only in this slice: no model execution, data writing or keyed/enum-axis implementation.
- Preserve the contract's checked numeric rules, prevalidation, generator order, explicit native projection and known-Boolean filters.
- The evaluator cannot depend on the Zincite CST; its frontend lowers to the shared representation.

## Done when

- [x] Zincite report requests accept supported source/let/report forms and MiniZinc-style positional comprehensions, including dependent generators, multiple binders, single-value generators and filters.
- [x] Scope, field/tuple/array access, short-circuit conditionals, checked scalar operations and explicit collection construction run in Zirql.
- [x] Reports retain Zincite's tagged native-value envelope and source limitations without inventing source ranges for generated data.
- [x] Unknown conditions, unsupported forms, invalid types, missing access, overflow and division failures are located; skipped runtime branches do not execute.
- [x] Candidates before filtering, nested results, copying and rendering consume the same aggregate request allowance; a failed request emits no partial report.

## Validation

- Use the brief's compact positional report, dependent-generator ordering, scope, skipped-error, arithmetic and pre-filter limit controls through the library and CLI.
- Run required checks in Zirql and Zincite; no full comprehension syntax matrix.

## Result

Zincite positional source/let/report construction now uses Zirql for checked scalar expressions, scopes and MiniZinc generators, preserving native reports, source limitations, located failures and one aggregate allowance. Canonical consumer source c4a3e568eaca852d2df4e7cae3bd7033218438e6 and separate records 12b821fb942d1f1b1e64a1b54f5471045102d768 are verified, installed and pushed; transport 02ca8d7e881b64666daa8845306f9372cb7bbf73. Core be8bd8b07d5e7e4fb148f8f2e032ffe0f5e226b9 has exact usable fallback =0.1.0-alpha.4. Zincite base-126 remains open for keyed reports and base-127 stays attended.

Validation:

- Independent whole-task review passed; framework, both consumer repositories and source transport remained unchanged.
- Required Zirql and canonical Zincite formatting, warning-free Clippy, workspace tests, diff and zdev checks passed.
- Focused public library and both CLI controls passed for ordering, projections, scope, static and skipped runtime branches, numeric/Input/BOM refusals, semantic limitations, pre-filter limits and buffered rendering.
- Final installed command bytes match the canonical committed build; both installed query forms preserve positional/native output and empty output on located budget failure.
- Locked actual registry adapter runs with exact alpha.4, the verified published checksum and no core Git/path override; non-Zirql lock entries are retained.
