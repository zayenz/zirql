+++
schema_version = 1
id = "base-060"
key = "decide-broader-zincite-language-alignment"
area = "base"
status = "done"
complexity = "advanced"
priority = "normal"
blocked_by = []
+++
# Define broader Zincite program and shared-operation alignment

## Outcome

An accepted language contract and bounded follow-up implementation scopes cover Zincite alignment with Zirium across whole programs and general native query operations.

## Context

The user explicitly requires alignment broader than let statements. The current brief records intermediate bindings/pipelines but has not settled all consumer syntax or new native-operation semantics. Inspect committed Zincite base-140 when available, existing direct native composition and quoted query wrappers, and Zirium query-language.md alongside Zirql readers/programs. Reconcile 044/046 control-flow integration, completed047, native048–054, and approved055–057 before proposing additional work. This task plans concrete remaining adoption, not a new query framework.

## Boundaries

- Design and planning only; no source implementation or expansion of active tasks. Let is Zincite's single binding keyword under the updated user decision; do not restore source aliases. Preserve existing data-edit, completeness and effect policies unless a separate explicit user choice changes them.
- Keep MiniZinc model resolution and MLIR semantics host-owned. Reuse existing shared representations and operations; no universal grammar configuration, operation registry, JSON interchange or speculative optimizer.

## Done when

- [x] Compare complete programs for selection reused by multiple statements, a constructed report pipeline, and a correlated native report. Show current and proposed Zincite/Zirium forms, covering direct stages versus query-string wrappers, grouping and precedence, immutable binding references, common function/control forms and explicit/final reporting. Identify which differences aid the domain and which are unnecessary frontend restrictions.
- [x] Produce a compact adoption table for shared native ordering/all-ties extrema (049), set algebra (050), map construction/ranking (051/052), correlated existence (053) and fixed points (054), plus any already-supported functions/control forms needing only integration. For each, identify intended Zincite input/result kinds, host primitive requirements, identity/order, empty/tie rules, Unknown/incomplete behavior and effect restrictions; mark actual nonapplicability with a concrete reason.
- [x] Recommend shared names, syntax and semantic rules wherever they fit. Present only material unresolved choices to the user, then record accepted decisions in the base brief and affected contract documentation. Do not silently inherit Zirium legacy JSON/comparison quirks or weaken Zincite original-input/atomic-edit rules.
- [x] After acceptance, record a bounded implementation split separating missing shared machinery from consumer adoption, with true prerequisites and a few executable acceptance examples. Reuse existing tasks and the two alignment integration tasks; do not recreate their work. Completion requires accepted decisions and scopes ready for subsequent task creation, not newly imported implementation tasks.

## Validation

- Check proposed examples against both actual frontend grammars and shared API contracts; clearly label unimplemented syntax. Use focused source/contract inspection, not a whole-codebase audit. No new product tests are expected. Run git diff --check and zdev check base --format json plus required documentation checks for changed guides.

## Result

Recorded accepted whole-program and native-operation alignment, insertion-ordered sortable dictionaries and sets, typed keys/extrema and bounded shared/consumer follow-up scopes.

Validation:

- Independent whole review passed with unchanged task snapshot and exact owned/foreign hashes.
- Complete-program comparisons and adoption table checked against actual core and committed host contracts; all required Rust, zdev, diff and site checks passed, with 910 internal links and rendered guide inspection.
