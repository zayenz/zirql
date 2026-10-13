# Host integration

To integrate a domain tool, choose its native item type and retain the context
that owns those items. The adapter supplies predicates or navigation; Zirql
runs the shared language constructs. Rendering is a separate call.

## Evaluation entry points

| Need | Shared API | Host supplies |
| --- | --- | --- |
| Ordered query stages | `Host` and `evaluate` | Native input, predicates, identity and typed stages |
| Bindings and reports | `ProgramHost` and `evaluate_program` | Native values, stage/set payloads, copying and emitters |
| Scalar or collection expressions | `evaluate_value` | Located expressions and immutable constructed bindings |

Use the entry point that fits the values your tool returns. The
[borrowed-word example](../README.md#a-borrowed-source-query) implements `Host`
for string slices and keeps them borrowed during evaluation.
Its [native controls](../tests/native_pipelines.rs) demonstrate retained contexts.

## Parse and lower

For the common pipeline syntax, call `parse_pipeline` with a predicate-reader
callback. For existing or domain-specific syntax, lower directly to the located
Rust representation. Do not serialize a parse tree just to cross this boundary.

The core validates the structure it can see. The frontend must validate references
inside opaque payloads, stage/result applicability and purity before executing
host operations. Check applicability even for empty streams: an empty input must
not make an invalid stage valid.

Store half-open UTF-8 ranges from the original query. Host-source locations also
need the document identity supplied by the host. Generated values have expression
locations, rather than invented ranges in an input file.

## Implement domain hooks

For pipelines, `Host` supplies predicate leaf evaluation, optional typed stage
execution and identity/tally keys. Hooks receive mutable retained context and
the active budget. The default stage and collection-item work charges are one
unit each; override them only when that matches your domain's actual work.
Charge scans and traversal that happen inside a hook, including nested work.

For programs, `ProgramHost` owns native value copying, stage execution, set
combination, native record construction and emission. `copy_value` need not
copy an entire document. It must admit the new result representation before
allocating it. `construct_record` receives already-admitted field slots.

A borrowed selection can remain borrowed while its source lives. A document
handle remains tied to its owner. Stale-handle checks, diff-side identity and
source-edit safety stay in the host. Two operations with equal printed names are
not necessarily the same operation.

## Domain policy

Zincite owns MiniZinc member resolution, data shape and target validation.
Zirium owns MLIR dialect semantics, operation identity and live edit behavior.
Zirql has no dependency on either consumer crate.

The core can represent generic sparse keyed values. A consumer may support a
narrower subset. Validate that subset at the boundaries required by the consumer;
a final rendering check alone is insufficient when intermediate values also
must obey host shape rules. Model execution and source writes are separate host
operations with their existing safety requirements.

## Library results and rendering

Return `NativeResult` or the host's value type to library callers. An emitter can
observe the value at a written position, but its buffering and transaction rules
are explicit host policy. A pure scalar expression must not emit or edit.

For signatures and budget charging helpers, use
[the Rust API reference](https://docs.rs/zirql) for your published version or
`cargo doc --open` for the pinned revision. The [diagnostics guide](diagnostics-and-limits.md)
explains how core and native errors remain distinct.
