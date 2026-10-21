+++
schema_version = 1
id = "base-123"
key = "consolidate-library-docs"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = ["base-117", "base-115", "base-121", "base-122", "base-128", "base-129", "base-130", "base-131", "base-132", "base-133"]
+++
# Make the Zirql library reference authoritative across all three sites

## Outcome

Host guides focus on bindings, native operators/functions and domain recipes, linking shared semantics and renderers to the organized Zirql library reference.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. The reference created by base-112 owns common signatures and empty/error/order rules. Zincite docs/queries.md around 804 and Zirium docs/query-language.md around 654/690 duplicate those contracts. Runtime tasks update docs when they land; this task removes remaining duplicate exposition and organizes cross-site navigation after actual adoption. Read Complete the common language boundary in brief.md and include the new common stages, native calls, program reader, constructed embedding and scalar conversions. Read Coherent native and constructed operations in brief.md and the outcomes of base-130 through base-133; organize the reference around their implemented common contracts.

## Done when

- [x] Each paired common operation has one reference entry covering its implemented native and constructed forms. Remove duplicate native labels and obsolete divergent policies; keep necessary default projections, native identity/result shape and explicit indexed-shape distinctions together in that entry. Native capture matching and value matching remain distinct contracts.
- [x] Both host sidebars expose the Zirql library reference; shared-function/operator/rendering sections link directly to appropriate entries and replace duplicated detailed contracts with concise host context.
- [x] Retain runnable MiniZinc/MLIR recipes, native projections/operators, accepted host syntax, version/support notes, output envelopes and edit/emission restrictions. All adopted extensions, scalar conversions and JSON/Markdown formats are described accurately.
- [x] The embedding guide describes the reduced host responsibilities and actual remaining hooks, links the simplified CSV/words embedding path and parameterized/function/numeric CSV recipes, and keeps scalar/string operations easy to find in the sidebar; public docs do not imply CP or unimplemented proposed forms are available.
- [x] Local generated sites and cross-project link targets are checked against the intended builds; the task completes without requiring live publication.

## Validation

- Run existing content preparation and site builds/link checks for changed sites, inspect sidebar and representative reference/recipe pages, git diff --check and zdev check base --format json. No new product tests.

## Result

Made the Zirql reference authoritative for common operations across all3sites, with one entry for native/constructed forms and direct purpose-based sidebar links. Shortened duplicated host exposition while retaining native projections/identity/resultshape/indexedconstraints, domain recipes, versionnotes, output/completeness and edit/emission rules. Embedding/CSV/words describe actual ConstructedProgram/sharedreader/preparation/nativecall/rendering boundaries. Fixed existing duplicatehref sidebarfragment navigation using supported Starlight links; documented existingarrayconcatenation. Consumer commits Zincite67552678e5f16f8598a536c31b0ce0f2ce1e323a and Ziriumc9c7f8a2af17f63c979d15b2364c0241a88c912d; runtimepin49c30dac unchanged.

Validation:

- Independent whole-task review,2staleprosefindings independentlyvetted, correction1 and freshfocusedPASS atW04d19e844fa90157; exactcore andall4consumerfieldsequal.
- All3content/check/build0diagnostics;33/22/17pages; local6368/1252/896 and642cross-projectlinks against72intendedpagesPASS. Desktop/mobile reference/CSV/embedding/host/diffrecipes andactualsidebarfragmentclicksPASS;33corepages0duplicatehref. Docs/configonly; prioracceptedruntimeassuranceretained.
