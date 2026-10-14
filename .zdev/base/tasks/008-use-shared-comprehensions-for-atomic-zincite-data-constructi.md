+++
schema_version = 1
id = "base-008"
key = "constructed-data-edits"
area = "base"
status = "done"
complexity = "advanced"
priority = "normal"
blocked_by = ["base-007"]
+++
# Use shared comprehensions for atomic Zincite data construction

## Outcome

An explicitly approved host contract creates or replaces structured .dzn assignments with one validated candidate using Zirql constructed values.

## Context

Read Zincite base-127 and docs/query-comprehension-contract.md completely. The user explicitly approved that exact bounded data contract on 2026-10-13 and chose emit as its terminator; the old emit_document spelling remains an alias. Zincite implemented it in 4b4246d543b612a4b94264f96411a78b2dc16f74, with the naming change in ae1ccdd9958ad633f20f6baf5d9665ce20101ef8, and base-127 records its completed acceptance. This resolves the existing attended gate for those exact choices. Verify and reconcile the current integration rather than implementing it twice. Prior framework approval still does not approve any broader grammar or policy. Reuse the shared evaluator and Zincite's existing candidate preparation/reparse/diff/write handling.

## Boundaries

- Use only the explicitly approved finite subset. No solver execution, arbitrary model evaluation, guessed dependent mappings or mutation of model/include files.
- Read all sources from the original input; validate coherent candidate assignment types/axes before one atomic replacement.
- Keep source insertion via set_value distinct from evaluated construction; preserve unrelated bytes and refuse unresolved comment placement.

## Done when

- [x] The exact host contract choices are explicitly approved and recorded before code changes; remaining user choices keep this task attended.
- [x] Library and both CLI forms create absent assignments and replace existing structured RHS values with coherent multi-assignment candidates using Zirql values.
- [x] A public example doubles an entity collection and explicitly remaps every dependent collection/reference with consistent index identities; numeric scaling alone does not satisfy it.
- [x] Ownership, target type/shape, duplicate targets, comments/directives, overlaps and candidate reparsing follow the approved policy.
- [x] A failed or incomplete multi-assignment request produces no partial output or write; complete candidates preserve unrelated source bytes.
- [x] Zincite base-127 records its actual integration evidence without claiming framework construction alone completed data-edit behavior.

## Validation

- Reuse shared construction controls plus one self-contained doubled-instance control and a failed atomic-write case.
- Run MiniZinc instance-only acceptance on generated public data, preservation checks and required Rust/record checks; no solver or private-input copying.

## Result

Verified the existing approved Zincite shared-comprehension data integration against current sources, without duplicating its evaluator or product tests. Reconciled the already-resolved attended gate and completed base-127 evidence; corrected only the documented source-before-let edit ordering in Zincite 3b9869ab96bbc838cdc33539aee0407b84e52af5.

Validation:

- Independent whole-task PASS with zero findings for all six conditions. Required formatting, warning-fatal Clippy and workspace tests passed: 44 Zirql tests and 439 Zincite tests; both local commands rebuilt.
- Library and both CLI forms produced identical 257-byte doubled and 164-byte all-create candidates. Native references/keys/groups/matrix/memberships, preview/diff/write, original bytes, permissions and failed/incomplete atomic-write buffering passed.
- MiniZinc 2.10.1 build 33348285743 accepted the actual doubled candidate with --instance-check-only. Git pins remain cef5216; model/include files and unrelated changes preserved; diff and zdev checks passed.
