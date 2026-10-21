+++
schema_version = 1
id = "base-128"
key = "constructed-program-embedding"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-117", "base-120"]
+++
# Provide a constructed-program API and simplify CSV and words

## Outcome

CSV and words use a concrete Zirql data-binding/program API, and CSV supports parameters and named functions without implementing native host traits.

## Context

Read brief.md sections Consolidate language machinery and data rendering (2026-10-21) and Complete the common language boundary (2026-10-21), including their compatibility, documentation and validation contracts. examples/csv.rs ValueReaderHost/ValueQueryHost and examples/words.rs repeat native-query refusals and binding plumbing. CSV Table::types duplicates its stored row declaration; docs/csv-host.md tells loader authors to edit both. src/value.rs function_facts and runtime_types.rs contain reusable declaration logic. evaluate_functions currently installs NoQueries and lacks current-input/shared-binding forwarding; combine the common reader and preparation from predecessors instead of switching calls blindly.

## Boundaries

- Keep file access, CSV decoding, header checks, lossless string cells, input limits and sole/named-table selection in the example. Reuse existing immutable owners and budget accounting.
- Use one concrete constructed-data adapter in the existing crate, with optional native extension remaining separate. Functions receive data explicitly and do not capture ambient tables, parameters or current input.

## Done when

- [ ] The public API accepts named constructed values with declared schemas and optional current input, then reads/prepares/runs the common function/parameter/ordered-binding program with one continuing budget.
- [ ] Declared schemas and values are supplied together; compact facts derive from the declaration without scanning every row or duplicating header schemas. Header-only unknown-column validation and immutable table reuse remain intact.
- [ ] CSV and words delete native refusal traits and duplicate binding/type bookkeeping. CSV retains rows, implicit operations such as count, named tables and its useful ambiguity diagnostics.
- [ ] CSV supports typed param declarations and named functions, including pattern parameters and existing bounded recursion, through the common machinery. Repeated --param name=value arguments use admit_parameters; existing positional query/input usage remains valid and table name=FILE arguments stay distinct.
- [ ] CSV uses the already-shared JSON/Markdown renderers and returns no partial output after conversion, evaluation or rendering failure.
- [ ] Rewrite docs/csv-host.md around load, declare/bind, read/prepare/run and render. Update docs/csv.md, words documentation, Rust API docs and embedding navigation with a minimal example and a runnable parameterized named-function CSV report in both formats.

## Validation

- Reuse current CSV and words controls; demonstrate a named function plus parameter with implicit/named inputs, a header-only unknown field refusal, and one later error with buffered output. Reuse existing function isolation/limit controls; add only a missing adapter-boundary case.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.
