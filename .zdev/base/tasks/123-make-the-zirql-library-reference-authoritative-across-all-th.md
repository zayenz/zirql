+++
schema_version = 1
id = "base-123"
key = "consolidate-library-docs"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-117", "base-115", "base-121", "base-122", "base-128", "base-129"]
+++
# Make the Zirql library reference authoritative across all three sites

## Outcome

Host guides focus on bindings, native operators/functions and domain recipes, linking shared semantics and renderers to the organized Zirql library reference.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. The reference created by base-112 owns common signatures and empty/error/order rules. Zincite docs/queries.md around 804 and Zirium docs/query-language.md around 654/690 duplicate those contracts. Runtime tasks update docs when they land; this task removes remaining duplicate exposition and organizes cross-site navigation after actual adoption. Read Complete the common language boundary in brief.md and include the new common stages, native calls, program reader, constructed embedding and scalar conversions.

## Done when

- [ ] Both host sidebars expose the Zirql library reference; shared-function/operator/rendering sections link directly to appropriate entries and replace duplicated detailed contracts with concise host context.
- [ ] Retain runnable MiniZinc/MLIR recipes, native projections/operators, accepted host syntax, version/support notes, output envelopes and edit/emission restrictions. All adopted extensions, scalar conversions and JSON/Markdown formats are described accurately.
- [ ] The embedding guide describes the reduced host responsibilities and actual remaining hooks, links the simplified CSV/words embedding path and parameterized/function/numeric CSV recipes, and keeps scalar/string operations easy to find in the sidebar; public docs do not imply CP or unimplemented proposed forms are available.
- [ ] Local generated sites and cross-project link targets are checked against the intended builds; the task completes without requiring live publication.

## Validation

- Run existing content preparation and site builds/link checks for changed sites, inspect sidebar and representative reference/recipe pages, git diff --check and zdev check base --format json. No new product tests.
