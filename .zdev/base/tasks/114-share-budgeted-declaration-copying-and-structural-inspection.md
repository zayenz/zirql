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
# Share budgeted declaration copying and structural inspection

## Outcome

Both hosts use Zirql to copy and inspect Zirql-owned declaration structure while retaining native admission and storage choices.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Compare src/runtime_types.rs copy_declaration/contains_native with Zincite comprehension.rs copy_declaration, program/worklists.rs declaration, and Zirium query/worklists.rs declaration and functions.rs contains_native. Current result-allocation charging differs. Zincite native_declaration intentionally classifies some native wrappers as constructed and is not equivalent to contains_native.

## Done when

- [ ] A documented public budgeted declaration-copy operation reconciles work, result allocation and nesting checks before allocation, preserving empty element contracts and record order.
- [ ] Both consumers use the shared copy operation and remove duplicate declaration-tree copying; structurally equivalent inspection uses common helpers while differing host classifications stay explicit.
- [ ] Rc/Arc wrappers, nominal/native meaning and host admission remain intact; API and host integration docs explain the boundary.

## Validation

- Reuse declaration/worklist tests; cover one nested record/sequence contract, preserved empty type and exhausted budget without a matrix.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
