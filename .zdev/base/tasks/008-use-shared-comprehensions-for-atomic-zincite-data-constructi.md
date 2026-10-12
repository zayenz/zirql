+++
schema_version = 1
id = "base-008"
key = "constructed-data-edits"
area = "base"
status = "open"
complexity = "advanced"
priority = "normal"
blocked_by = ["base-007"]
+++
# Use shared comprehensions for atomic Zincite data construction

## Outcome

An explicitly approved host contract creates or replaces structured .dzn assignments with one validated candidate using Zirql constructed values.

## Context

This is an attended contract task. Read Zincite base-127 and docs/query-comprehension-contract.md completely. Before implementation, present the exact source/let/create/replace/batch grammar, operator/helpers, target/model ownership, keyed/empty-axis and comment policy for the user's existing data-edit gate. Prior framework/task approval does not approve those outstanding choices. Reuse the implemented shared evaluator and Zincite's existing candidate preparation/reparse/diff/write handling.

## Boundaries

- Use only the explicitly approved finite subset. No solver execution, arbitrary model evaluation, guessed dependent mappings or mutation of model/include files.
- Read all sources from the original input; validate coherent candidate assignment types/axes before one atomic replacement.
- Keep source insertion via set_value distinct from evaluated construction; preserve unrelated bytes and refuse unresolved comment placement.

## Done when

- [ ] The exact host contract choices are explicitly approved and recorded before code changes; remaining user choices keep this task attended.
- [ ] Library and both CLI forms create absent assignments and replace existing structured RHS values with coherent multi-assignment candidates using Zirql values.
- [ ] A public example doubles an entity collection and explicitly remaps every dependent collection/reference with consistent index identities; numeric scaling alone does not satisfy it.
- [ ] Ownership, target type/shape, duplicate targets, comments/directives, overlaps and candidate reparsing follow the approved policy.
- [ ] A failed or incomplete multi-assignment request produces no partial output or write; complete candidates preserve unrelated source bytes.
- [ ] Zincite base-127 records its actual integration evidence without claiming framework construction alone completed data-edit behavior.

## Validation

- Reuse shared construction controls plus one self-contained doubled-instance control and a failed atomic-write case.
- Run MiniZinc instance-only acceptance on generated public data, preservation checks and required Rust/record checks; no solver or private-input copying.
