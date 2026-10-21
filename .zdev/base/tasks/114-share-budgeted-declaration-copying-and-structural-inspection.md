+++
schema_version = 1
id = "base-114"
key = "shared-declaration-metadata"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-113"]
+++
# Share declaration metadata and scalar argument admission

## Outcome

Both hosts use Zirql to copy and inspect Zirql-owned declaration structure and admit common scalar arguments while retaining native admission and storage choices.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Compare src/runtime_types.rs copy_declaration/contains_native with Zincite comprehension.rs copy_declaration, program/worklists.rs declaration, and Zirium query/worklists.rs declaration and functions.rs contains_native. Current result-allocation charging differs. Also read Complete the common language boundary in brief.md. Compare runtime_types.rs check_known_type/check_type_facts with Zincite program/functions.rs check_argument and language.rs ArgumentKind::admits, and Zirium query/report.rs scalar_type_is. Both hosts also repeat checked nonnegative int-to-usize extraction. Zincite native_declaration intentionally classifies some native wrappers as constructed and is not equivalent to contains_native.

## Done when

- [ ] A documented public budgeted declaration-copy operation reconciles work, result allocation and nesting checks before allocation, preserving empty element contracts and record order.
- [ ] Both consumers use the shared copy operation and remove duplicate declaration-tree copying; structurally equivalent inspection uses common helpers while differing host classifications stay explicit.
- [ ] A narrow public scalar type-fact admission API handles declared wrappers and explicit strict/deferred facts; both consumers replace equivalent scalar checks while preserving deliberate policy differences. Reuse existing runtime admission rather than adding a parallel checker.
- [ ] A located nonnegative integer-to-usize conversion replaces the two equivalent host argument conversions, preserving overflow refusal and host diagnostic context.
- [ ] Rc/Arc wrappers, nominal/native meaning and host admission remain intact; API and host integration docs explain declaration copying, scalar admission, deferred policy and the remaining domain boundary.

## Validation

- Reuse declaration/worklist tests; cover one nested record/sequence contract, preserved empty type and exhausted budget without a matrix. Reuse host scalar-argument controls and add only a missing declared/deferred or negative/overflow refusal.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
