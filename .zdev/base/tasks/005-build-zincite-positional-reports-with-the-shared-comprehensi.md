+++
schema_version = 1
id = "base-005"
key = "minizinc-comprehension-reports"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-004"]
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

- [ ] Zincite report requests accept supported source/let/report forms and MiniZinc-style positional comprehensions, including dependent generators, multiple binders, single-value generators and filters.
- [ ] Scope, field/tuple/array access, short-circuit conditionals, checked scalar operations and explicit collection construction run in Zirql.
- [ ] Reports retain Zincite's tagged native-value envelope and source limitations without inventing source ranges for generated data.
- [ ] Unknown conditions, unsupported forms, invalid types, missing access, overflow and division failures are located; skipped runtime branches do not execute.
- [ ] Candidates before filtering, nested results, copying and rendering consume the same aggregate request allowance; a failed request emits no partial report.

## Validation

- Use the brief's compact positional report, dependent-generator ordering, scope, skipped-error, arithmetic and pre-filter limit controls through the library and CLI.
- Run required checks in Zirql and Zincite; no full comprehension syntax matrix.
