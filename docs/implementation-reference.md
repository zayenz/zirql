# Implementation reference

To run the [word tutorial](examples.md) in an application, load an array of
strings, expose it as `words` and pass the user's expression to Zirql. The
application then handles the result. The [complete Rust
example](getting-started.md) shows this integration.

For an application over files, [Build the CSV host](csv-host.md) explains how to
load declared rows, expose bindings and run a program before writing JSON or Markdown.
Its [usage guide](csv.md) shows the input tables, queries, results and commands
together.

An application can also expose its own objects and operations. This reference
explains the Rust adapter interfaces, then follows the word model into native
occurrences, source ownership and navigation.

## Choose a data model

| Your input | Useful representation | Application responsibilities |
| --- | --- | --- |
| Words, numbers or report rows | Constructed arrays and records | Load data, expose bindings, handle results |
| Source occurrences | Records with offsets, or borrowed native items | Preserve source ownership and coordinates |
| Syntax nodes or operations | Native handles | Define identity, predicates, navigation and projections |
| Domain enums and indexed data | Nominal members and keyed arrays | Resolve identities and enforce domain shape rules |

Constructed values often suffice. The adjacent-pair expression needs only an
array of strings. Native values let queries retain references to existing
objects and use operations that need their document or model context.

The [adapter examples](semantic-models.md) explain Zincite and Zirium's choices
at these boundaries. Usage instructions belong to their own documentation.

## Connect text to evaluation

The library has several entry points, each suited to a different integration:

| Entry point | Use |
| --- | --- |
| `ConstructedBinding` / `ConstructedProgram` | Admit declared data, bind names/current input and run common programs without native traits |
| `read_value_expression`, `read_value_expression_in_scope` and `ValueReaderHost` | Parse shared expressions and delegate domain syntax |
| `evaluate_query_value` and `ValueQueryHost` | Evaluate constructed data with explicit binding reads and native leaves |
| `parse_pipeline` and `Host` | Embed the small filter/head/count pipeline reader |
| `ProgramHost` and `ProjectionHost` | Execute native programs and correlated report fields |
| Located Rust expression and program types | Integrate an existing parser or build queries directly |
| `FunctionRequest` / `PreparedFunctions` | Prepare typed pure functions and evaluate them with the active host |

Words and CSV use `ConstructedBinding` and `ConstructedProgram`. Supply each
value with its declaration, read the common program, then evaluate with borrowed
bindings, optional current input and `ParameterInputs`. Compact schema facts
retain declared empty fields and positional meaning. The
[minimal example](embedding-programs.md#constructed-data-programs) shows the call
sequence.

Use the native interfaces for handles, domain operations and host effects.
Functions in both routes receive all values through explicit arguments.

[Embedding in Rust](host-integration.md) describes hook obligations, native
ownership, validation and projection. [Construct programs in Rust](embedding-programs.md)
shows program builders, parameters, functions and control-flow representations.
The [value evaluation reference](value-runtime.md) covers expression nodes,
type facts and owned results. Generate `cargo doc --open` for signatures at your
dependency revision.

## Define domain operations

A host can add a predicate such as `prefix`, a navigation step such as
`next_word`, or a projection such as `text`. It parses the operation's arguments
into a typed payload and supplies its meaning during evaluation.

Keep generic language behavior in Zirql. For example, a host supplies direct
edges and identity for path traversal; Zirql finds a bounded shortest witness.
A host supplies a string projection; Zirql groups and sorts the resulting rows.
The [word-model extensions](language-direction.md) show how to choose these
operations without making them part of the base language.

The shared reader owns generic query grammar, including common and MiniZinc
comprehensions, records, conditionals and indexed literals. Domain arguments
reenter the scoped expression reader. A narrow `parse_domain_call` hook handles
signatures that require native/model facts, such as enum member resolution.
The [adapter call sequence](host-integration.md#shared-query-grammar-and-domain-calls)
keeps full-AST admission and actual input-file parsing with the host. Typed
callers can also build located Rust trees directly.

## Preserve ownership and enforce limits

Native handles retain their source or document owner. Equal printed names do
not establish identity. The host rejects stale or foreign handles and decides
which facts are complete enough to use.

Pass one budget through parsing, validation, native callbacks, copying and
rendering. Charge host scans and admit allocations before doing the work. Nested
evaluation continues with the remaining allowance. The
[budget and diagnostic API reference](runtime-limits.md) describes what the
core charges and what the host must account for.

## Return data and control effects

Results can remain Rust values. The application chooses when to call Zirql's
[JSON renderer](library-rendering.md), inspect returned records or use native
handles in another operation. The evaluator does not require a JSON conversion
between stages.

If an application supports edits, it owns their legality and commit policy.
Pure expression and function contexts must reject effects during preflight.
Applications that promise all-or-nothing output must buffer or validate before
publishing it. See [library results and rendering](host-integration.md#library-results-and-rendering).

## Understand the implementation

The [execution and cost guide](computation.md) explains what happens within a
request, where values share storage and when results need copying. Read it when
sizing a request budget or investigating an expensive query. A separate
[measurement report](execution-measurements.md) records a comparison of two
evaluator revisions, including their small-query overhead.
