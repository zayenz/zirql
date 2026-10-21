+++
schema_version = 1
id = "base-128"
key = "constructed-program-embedding"
area = "base"
status = "done"
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

- [x] The public API accepts named constructed values with declared schemas and optional current input, then reads/prepares/runs the common function/parameter/ordered-binding program with one continuing budget.
- [x] Declared schemas and values are supplied together; compact facts derive from the declaration without scanning every row or duplicating header schemas. Header-only unknown-column validation and immutable table reuse remain intact.
- [x] CSV and words delete native refusal traits and duplicate binding/type bookkeeping. CSV retains rows, implicit operations such as count, named tables and its useful ambiguity diagnostics.
- [x] CSV supports typed param declarations and named functions, including pattern parameters and existing bounded recursion, through the common machinery. Repeated --param name=value arguments use admit_parameters; existing positional query/input usage remains valid and table name=FILE arguments stay distinct.
- [x] CSV uses the already-shared JSON/Markdown renderers and returns no partial output after conversion, evaluation or rendering failure.
- [x] Rewrite docs/csv-host.md around load, declare/bind, read/prepare/run and render. Update docs/csv.md, words documentation, Rust API docs and embedding navigation with a minimal example and a runnable parameterized named-function CSV report in both formats.

## Validation

- Reuse current CSV and words controls; demonstrate a named function plus parameter with implicit/named inputs, a header-only unknown field refusal, and one later error with buffered output. Reuse existing function isolation/limit controls; add only a missing adapter-boundary case.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.

## Result

Added ConstructedBinding and ConstructedProgram over common reading/preparation/execution, compact declared facts and isolated CurrentInput. CSV/words remove native refusal and duplicate binding/schema plumbing. CSV supports typed parameters and named functions with explicit JSON/Markdown buffering. File decoding, exact cells, headers, input bytes and table policy remain local. API/embedding/sidebar docs and numeric report updated. Core-only, no consumer adoption or dependency-pin changes.

Validation:

- Independent whole-task PASS at Wffe6ac25f62e8ed7; exact core and read-only consumer Git fields equal; no corrections.
- Rust fmt/Clippy all targets/201 tests/12 doctests and example builds pass; sole/named numeric reports both formats/repeated params, three buffered failures and seven words commands pass; site check/build33pages6430links pass. Existing query_profile smoke1passed with temporary metadata exactly restored, no new clone or committed pin.
