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
sets and native capture fields and refuses effects and emitters. Domain hooks validate primitive
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

## Native match adapter

Implement `MatchHost<C>` on the existing `ProjectionHost<C>` adapter. Its
`Guard` type holds a domain Boolean leaf. `validate_guard_leaf` checks every
native operand against the supplied input kind and ordered scope, including
purity and supported types. `evaluate_guard_leaf` returns the existing `Truth`
with the active context, immutable bindings and budget. The core owns all
Boolean wrappers and short-circuit evaluation; the leaf owns native identity.

The callable pair is `match_types(&matching, host, &input_kind, &scope, budget)`
and `evaluate_match(&matching, host, input, context, bindings, budget)`.
Type discovery returns ordered `NativeCapture<Kind>` fields. Execution returns
`Vec<Value>` rows built through `ProgramHost::construct_record`. Keep the
schema in the native collection wrapper, including when the row vector is
empty. Wrapping already-admitted rows moves them; admit any new host container
before allocation.

`ProjectionHost::subject_kind` declares the singleton kind yielded by
`subjects` and rejects unsupported enumeration even on empty input. Its default
returns the input kind for existing hosts. Both `from` and `bind` use these
hooks; the core checks the exact-one requirement for `bind`. Projection binds
that same singleton kind, so a collection of native capture records can feed
shared report projection without losing its field schema.

Expose the singleton record through the borrowed
`ProgramHost::capture_record` and `ProjectionHost::capture_record_kind` hooks.
Both default to absence. Hosts unwrap their wrappers; `NativeCapture::get`
checks and charges field lookup, and the expression runner uses `copy_value`
to retain the selected native type. Kinds should use shared immutable metadata
so their required `Clone` operation does not hide native materialization.
Handles never enter `ConstructedValue` or `ValueType`.

`ValueReaderHost::matching` wraps the shared parsed tree as a native stage.
Its default returns a located unsupported error. Supporting adapters dispatch
that stage to the shared match functions; `parse_stage` and
`parse_predicate_leaf` only parse domain primitive arguments. They must not
implement a second match evaluator.

The reader releases partially constructed shared trees when parsing fails.
Callers own successful returns and must release long flat native trees
iteratively: use `dispose_expression` for a native expression, `dispose_match`
for a match, and `dispose_native_value` for a shared value containing native
operands. For a returned projection, consume its fields with the same helper:

```rust
for (_, value) in projection.fields {
    zirql::dispose_native_value(value);
}
```

These consuming helpers need no remaining parsing allowance. They visit shared
wrappers and native composition; the host remains responsible for cleanup
inside opaque stage, set-operator, emitter and predicate payloads. Host wrappers
can call the helpers from their own `Drop` implementations.

Native handles still retain the host's document ownership, source coordinates,
completeness and diff-side rules. Operand lookup and SSA equality, for example,
remain Zirium operations. Rendering is an explicit host boundary and does not
supply identity to the matcher.

## Native path adapter

Implement `PathHost<C>` on the existing `ProjectionHost<C>`. Its `Target`
holds an ordinary domain predicate leaf. Keep it separate from any narrower
match guard language. `validate_path_edge` admits one pure native primitive,
and `validate_target_leaf` checks every operand, type and scope before scans.
The core also refuses edges declared effectful by `is_pure`.

`path_neighbors` returns admitted singleton neighbors in direct edge order.
An edge preserves the subject kind and native owner. It retains unresolved-edge
limitations and existing stale-handle or owner refusals. `same_path_identity`
compares real native identities after the core charges a comparison;
printed names and debug text cannot replace that identity.

The callable pair is `path_types(&paths, host, &input_kind, &scope, budget)`
and `evaluate_paths(&paths, host, input, context, bindings, budget)`.
Type discovery returns the ordered `NativeCapture<Kind>` schema for `source`,
`sink` and `steps`. Execution returns rows through `construct_record`, using
`subjects`, `subject_kind` and charged `copy_value` calls. Retain the schema
when wrapping an empty result, as for matching.

`path_steps` moves admitted singleton values into one ordered native selection.
Its declared kind agrees with `subject_kind`, and its sequence shape survives
a one-step witness. Charge any additional host allocation before constructing
it. The core owns breadth-first traversal, visited comparisons, predecessor
indices and witness reconstruction; adapters supply no second traversal.

In a text frontend, `parse_stage` calls `read_paths` with the original reader
and budget. `ValueReaderHost::parse_path_edge` receives one located identifier
and maps it to an existing typed primitive without consuming more syntax.
Its default refuses paths. Target leaves use `parse_predicate_leaf`; shared
Boolean wrappers remain with the common reader. Preserve host parameter
discovery and purity checks for native operands inside those leaves.

Release returned path trees with `dispose_paths`. It drops shared Boolean
wrappers iteratively; opaque edge and target payloads retain their host disposal
contract. Native results and later emitter snapshots use the same budget.
