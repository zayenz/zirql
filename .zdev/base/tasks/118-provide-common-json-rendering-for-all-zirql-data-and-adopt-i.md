+++
schema_version = 1
id = "base-118"
key = "shared-json-data"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Provide common JSON rendering for all Zirql data and adopt it in CSV

## Outcome

Zirql owns ordinary and tagged JSON conversion/rendering, and the CSV example uses the shared API.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. The rendering subsection settles ownership and encoding. Start with examples/csv.rs write_json/JsonBuffer, Zirium query/comprehension.rs to_json and program.rs BudgetWriter/serialize_json, and Zincite comprehension.rs tagged/tagged_key. Source inspection here does not authorize modifying consumers in this task; host integration follows separately.

## Boundaries

- Keep this slice in Zirql and its CSV example. No JSON importer, native identity reconstruction or serialization during evaluation.

## Done when

- [ ] Callable JSON conversion and compact/pretty buffered rendering cover every ConstructedValue kind under the brief’s ordinary/tagged contract, retaining dictionary key structure, insertion order, declared axes/missing cells and nominal metadata.
- [ ] Common byte writing/escaping/allocation accounting uses the active budget and supplied location; declared wrappers are transparent and limit/error paths return no partial buffered result.
- [ ] The CSV executable delegates ordinary output to Zirql and removes its recursive JSON writer and duplicate output-budget code; existing output remains compatible for previously supported data and nominal/keyed output follows the documented extension.
- [ ] Public API docs, JSON library entry/sidebar and a runnable CSV data-output example document both representations and explicit rendering.

## Validation

- Use focused fixtures spanning nested structural dictionary keys, ordered records/sets, nominal/keyed shape and budget failure; reuse CSV existing examples, compare parsed content and intentional ordering.
- Run Zirql Rust checks, documented CSV commands, git diff --check and zdev check base --format json; prepare/check affected docs links.
