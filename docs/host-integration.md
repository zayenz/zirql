# Embedding in Rust

To integrate a domain tool, choose its native item type and retain the context
that owns those items. The adapter supplies predicates or navigation; Zirql
runs the shared language constructs. Rendering is a separate call.

## Dependency and toolchain

Add Zirql to a Rust crate with Cargo. The current crate uses edition 2024 and
requires Rust 1.88 or newer. Development integrations pin a Git revision;
[getting started](getting-started.md#add-a-pinned-dependency) supplies one with
a runnable scalar expression. Check API documentation for that revision with
`cargo doc --open`.

A scalar expression needs a located expression tree, immutable bindings and a
budget. A query over your own data also needs a native item type, an owning
context and a host implementation. There is no standalone Zirql executable
that supplies these domain choices.

## Evaluation entry points

| Need | Shared API | Host supplies |
| --- | --- | --- |
| Ordered query stages | `Host` and `evaluate` | Native input, predicates, identity and typed stages |
| Bindings and reports | `ProgramHost` and `evaluate_program` | Native values, stage/set payloads, copying and emitters |
| Scalar or collection expressions | `evaluate_value` | Located expressions and immutable constructed bindings |

Use the entry point that fits the values your tool returns. The
[borrowed-word example](../README.md#a-borrowed-source-query) implements `Host`
for string slices and keeps them borrowed during evaluation.
For program construction, see [construct programs in Rust](embedding-programs.md).

## Retain native items and their owner

`NativeInput<Item, Context>` supplies the initial rows and their retained context.
`evaluate` returns `NativeResult<Item, Context>` with a stream, count or tally.
Items need not implement `Clone`, `Send`, `Sync` or `'static`. A string slice may
borrow source text; an operation handle may refer to a document retained by the
context. Keep that owner alive for as long as callers can use the results.

## Parse and lower

For the common pipeline syntax, call `parse_pipeline` with a predicate-reader
callback. For existing or domain-specific syntax, lower directly to the located
Rust representation. `Pipeline::new` accepts located typed stages;
`Program` and `ValueExpression` describe programs and scalar/collection expressions.
`read_value_expression` reads JSON-shaped values, operators, positional
comprehensions, reducers and requirements. `read_projection` reads an explicit
subject and ordered fields. A `ValueReaderHost` supplies one domain stage or
predicate leaf; the reader owns native composition and Boolean wrappers.
`read_statement_prefix` recognizes contextual `let name =` and `report` forms.
The host retains its program sequencing and emission policy.
Comprehension-local names take precedence over native stage names in value
expressions. The head sees every binder; each source sees earlier binders, and
each filter also sees its current binder.

`ArgumentReader` retains original UTF-8 byte offsets. It has word, integer and
quoted-string helpers that skip whitespace and `#` line comments. Quoted strings
retain literal `#` characters. `remaining` and `advance` let a predicate callback read
other argument forms. The callback must consume and validate the whole leaf.

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

## Keep one request budget

Choose limits at the request boundary and retain one mutable `Budget` through
input admission, preflight, execution, nested calls, copies and rendering.
Use `NativeInput::collect` to charge and admit incoming rows. For programs and
constructed values, `Budget::for_program` enables aggregate result accounting.
A new nested input does not receive a fresh allowance.

Host hooks charge scans and materialization that the core cannot see. Check known
sizes and admit result units before allocation. Choose limits for the actual
host request; Zirql supplies no defaults. The
[limits reference](diagnostics-and-limits.md) explains the accounting units.

## Constructed-value APIs

The callable entry points have different jobs:

| API | Purpose |
| --- | --- |
| `free_value_names` | Find external references using declared type facts |
| `validate_value_expression` | Check an expression against binding types |
| `constructed_value_type` | Discover type facts from an actual value |
| `evaluate_value` | Evaluate with immutable bindings |
| `copy_constructed_value` | Make a validated, budgeted snapshot |

`evaluate_value` validates actual binding types before execution. Explicit
preflight is useful before projecting domain data, but does not replace this
check. Keep these calls on the same budget, including any copies of bindings
or emission snapshots.

## Domain policy

The host defines its predicates, native identity, navigation and permitted
values. In the [word model](examples.md), `prefix` is an exact string test and
the input remains borrowed text. A document host can instead retain handles
and resolve their properties from its owning context. The
[Zincite and Zirium integrations](semantic-models.md) show how those choices
affect queries, completeness and rendering.

The core can represent generic sparse keyed values. A consumer may support a
narrower subset. Validate that subset at the boundaries required by the consumer;
a final rendering check alone is insufficient when intermediate values also
must obey host shape rules. The development API
`evaluate_value_with_host_validation` lets a trusted host check each successful
runtime value before its parent uses it, with the same budget and location.
It is absent from the published alpha.5 release; use a development revision that
contains it when this check is required.

Model execution and source writes are separate host operations with their existing safety requirements.

## Library results and rendering

Return `NativeResult` or the host's value type to library callers. An emitter can
observe the value at a written position, but its buffering and transaction rules
are explicit host policy. A pure scalar expression must not emit or edit.
The frontend must also reject emitters or edits hidden inside opaque payloads in pure bindings and nested
constructor operands. Core validation cannot inspect those payloads.

Specify whether written emission is terminal or pass-through, whether output is
buffered and what happens if a later operation fails. A CLI promising complete
output must buffer until the request succeeds. Library live edits and emitters
receive no automatic rollback from Zirql. Source-edit safety and transaction
rules belong to the host.

For signatures and budget charging helpers, use
[the Rust API reference](https://docs.rs/zirql) for your published version or
`cargo doc --open` for the pinned revision. The [diagnostics guide](diagnostics-and-limits.md)
explains how core and native errors remain distinct.

## Correlated native projection

`Projection<Q>` contains a located subject name and ordered fields of
`ValueExpression<Q>`. For native reports, `Q` is a normalized `Expression` tree.
Each subject is a singleton native selection. `evaluate_projection` preflights
every field before obtaining subjects, then evaluates the fields in written
order and returns an `Array` of `Record` values. Input duplicates remain separate
rows. Empty inputs still receive complete scope, type and purity checks.

`ProjectionHost` extends `ProgramHost` with native kinds, stage applicability
and effect facts, visible nested selector operands, subject extraction and
scalar conversion. `native_expression_kind` walks bindings, grouping, sequences
and sets and refuses effects and emitters. Domain hooks validate primitive
arguments and retain native source failures. Handles require an explicit domain
projection before becoming scalar report data.

`projection_types` and `native_value_types` retain inferred record alternatives,
including schemas of empty row arrays. Keep these facts with saved constructed
bindings so later filters and reducers can preflight known types.
`evaluate_native_value` uses the same shared value runner for literals,
constructed names and native operands. A false requirement preflights its value
operand without executing that operand's native query.

`Bindings::with_local` creates an immutable child scope by sharing outer values
and copying admitted name metadata. Native values need no deep `Clone` bound.
Reads still use the host's charged copy operation. Dropping a child scope
restores outer bindings on success and failure.
