# Embedding in Rust

An application supplies data, exposes the operations its users need and handles
the result. Start with constructed bindings when arrays and records describe the
input. Add native items when queries need source identity, navigation or access
to an existing model. The adapter is the Rust code connecting that model to
Zirql's traits. A frontend reads query text and builds the expressions that the
evaluator runs.

## Dependency and toolchain

Add Zirql to a Rust crate with Cargo. Zirql 0.2.0 uses edition 2024 and requires
Rust 1.88 or newer. The
[setup guide](getting-started.md#use-zirql-in-another-rust-project)
shows the registry dependency and a complete executable. Generate API
documentation for your selected dependency with `cargo doc --open`.

## Evaluation entry points

| Need | Shared API | Host supplies |
| --- | --- | --- |
| Text expressions over bindings | `ValueReaderHost`, `read_value_expression`, `ValueQueryHost`, `evaluate_query_value` | Binding names, type facts and budgeted value reads |
| Constructed expression trees | `evaluate_value` | Located expressions and immutable constructed bindings |
| Ordered native stages | `Host` and `evaluate` | Native input, predicates, identity and typed stages |
| Native bindings and reports | `ProgramHost` and `evaluate_program` | Native values, stage/set payloads, copying and emitters |
| Correlated report fields | `ProjectionHost` and `evaluate_projection` | Native subjects and pure projection operations |
| Named pure functions | `FunctionHost`, `FunctionRequest`, `PreparedFunctions` | Type resolution and pure native operations |

The [word example](getting-started.md) uses the first row. Its reader recognizes
one binding, `words`, and rejects native stages. Its value host returns a
budgeted copy of the string array. Zirql reads and evaluates the collection
stages as well as the comprehension, so this host needs no sort, unique or tally
hooks. This is enough to run the front-page examples.
Typed callers nest `ValueExpressionKind::Sort`, `Unique` and `Tally` nodes and
call `evaluate_value` without parsing text. The common reader lowers constructed
pipes to these same nodes; existing prepared and function entry points use the
same evaluator. See [value evaluation](value-runtime.md) for type discovery,
validation and owned results.

The [CSV usage guide](csv.md) uses the same value interfaces for a complete
application with [runnable source](../examples/csv.rs) and small
[people](../examples/csv/people.csv) and [teams](../examples/csv/teams.csv) fixtures.
Its adapter loads arrays of string dictionaries, supplies header type facts and
named-table bindings, and renders the final value as JSON. It reads ordered
`let` bindings and one final expression into a value block. Zirql evaluates
scope, correlated comprehensions, saved values and sorting; the CSV adapter
needs no separate implementations of those operations. Named functions need
the larger program interface below and are outside that example's input format.

[Build the CSV host](csv-host.md) explains the implementation in order, including
the header types and reader hooks. Start there to adapt it to your own tabular
data; use the sections below when you need native objects or domain operations.

The [borrowed-word example](../README.md#a-borrowed-source-query) instead
implements `Host` for string slices, retaining their source through evaluation.
The remaining sections explain how to add native data and operations. [Construct
programs in Rust](embedding-programs.md) covers the program builders.

## Retain native items and their owner

`NativeInput<Item, Context>` supplies the initial rows and their retained context.
`evaluate` returns `NativeResult<Item, Context>` with a stream, count or tally.
Items need not implement `Clone`, `Send`, `Sync` or `'static`. A string slice may
borrow source text; an operation handle may refer to a document retained by the
context. Keep that owner alive for as long as callers can use the results.

## Shared query grammar and domain calls

Start an `ArgumentReader` at the query's original byte offset and pass one
request `Budget` to `read_value_expression`. Functions, blocks, predicates and
both comprehension spellings use that reader. A domain argument that contains
an ordinary expression calls
`read_value_expression_in_scope(reader, host, locals, budget, nesting)`. Pass the
received lexical names and the remaining nesting allowance. These names take
precedence over stage names; they expose no caller values. Returned trees own
their names and native payloads, so the input text and name slices need live
only while reading.

The scoped stage, predicate-leaf, predicate-argument and constructed-filter
hooks receive `nesting` after their budget argument. Forward that allowance to
nested readers instead of supplying a fresh limit. `parse_domain_call` uses the
same contract. Zirql also caps each nested reader against the active request
allowance, so a new reader cannot reset it. Zirql restores the enclosing
allowance when the callback returns, on success or error. Work and result
charges remain spent.

`ValueReaderHost::parse_domain_call(callee, reader, locals, budget, nesting)`
handles a recognized domain signature after the shared reader consumes its
semantic callee. The cursor is at `(`. Return `None` without moving the cursor
for an ordinary shared function call. For example, a Zincite adapter can read
`enum_member(TypeIdentifier, "member")` with `word()` and `quoted_string()`, then
resolve the type and member against its model and return a located nominal
literal. The hook does not read an entire generic expression. Native stage and
predicate hooks still read their own operation arguments; ordinary value
arguments reenter the scoped reader.

After reading, the host admits the entire AST for its context, resolves native
and nominal facts, then prepares and executes through Zirql. Finally it renders
results or validates an edit candidate. A closed Data context must visit unused
bindings, every conditional/match arm, generators, declared/fallback axes and
expressions inside opaque native arguments before execution. Grammar acceptance
does not grant report helpers, functions, effects or edit capabilities.

The host keeps its input-file CST and literal-source decoding for actual model
and input syntax. Zirql reads query records, conditionals, indexing, matrices,
comprehensions and lexical forms. Zincite retains its Data admission policy and
model facts; Zirium retains MLIR/diff meanings and native ownership. Both use
the same expression grammar without a whole-expression CST hook.

## Inspect and transform shared syntax

Zirql owns expression child order and native-payload mapping. Use
`map_value_payloads(expression, mapper)` to move each opaque native payload
through a host conversion without a `Clone` bound. It preserves ranges,
patterns, generator selection flags, declarations and collection order.
`NativeQuery` remains a projecting leaf; `NativeValue` retains its native value.
The mapper uses explicit stacks, including for deep constructed trees.

`for_each_value_child` and `for_each_value_child_mut` inspect one node without
allocation or evaluation. Their order is written arguments and fields,
dictionary key before value, declared axes before generator sources and
filters, then comprehension key/head and zero-cell fallback axes. Fallbacks
remain separate AST fields so hosts can choose deferred discovery explicitly.
For bounded preorder inspection or rewriting, use `walk_value` or
`walk_value_mut` with the active request budget and remaining nesting allowance.
Their callbacks receive the node, its depth and that budget. Walks admit known
child storage before allocation and return located errors. They do not bind
names or evaluate branches. Native-wrapper discovery uses `walk_expression`;
input, binding and sequence wrappers add no logical nesting. Opaque stages
remain host callbacks.

For example, this renames a native payload while retaining its query range:

```rust
use zirql::{Located, ValueExpressionKind as K, map_value_payloads};
let expression = Located::new(K::NativeValue(Box::new("declarations")), 2..14);
let mapped = map_value_payloads(expression, str::to_owned);
assert_eq!(mapped.range, 2..14);
assert!(matches!(mapped.value, K::NativeValue(name) if *name == "declarations"));
```

Zincite uses these operations for native payload lowering, primitive recognition
and native discovery. Shared record evaluation calls its native construction
hook. Zincite keeps MiniZinc member resolution, source coordinates, scalar
projection, the closed Data subset and atomic edit checks.
Its eager native discovery skips `default` fallbacks; full written admission
still inspects them. Zirium uses the operations for report normalization and
native-root discovery. It keeps document/operation ownership, stale-handle
checks, dialect meanings, diff sides and effects. Both development integrations
adopt `map`, `flat_map`, selective generators, `windows`, `runs_by`, `with` and
`align` through the common reader and their existing builder entry points.
They pin the accepted Git revision; registry 0.2.0 predates these integrations.

## Scoped predicates

`read_predicate_argument(reader, host, locals, budget, nesting)` reads an optional
`pattern =>` followed by a shared value expression. The reader binds `_` only for
that argument. Predicate bodies use the shared expression reader, including both
comprehension spellings. The lexical names include the unnamed argument and
every pattern name. `_` retains the whole subject, including with a destructured
argument. Domain leaves keep their narrow argument signatures.

`read_native_predicate_argument(reader, host, locals, budget, nesting)` reads a
mixed native Boolean tree, returning `Predicate<H::Predicate>`. Use it for host
arguments such as root/ancestor tests or diff predicates that combine domain
leaves with shared expressions. `read_predicate_argument` returns a value-only
`PredicateArgument` for constructed filters and other shared value bodies.
Both readers start at the original argument cursor and leave the caller's
closing delimiter unread. They share lexical scope and cumulative allowances.

For native Boolean trees, `is_predicate_leaf` recognizes domain leaves without
consuming syntax or budget. `parse_predicate_leaf_in_argument` receives the
optional borrowed `Pattern` and lexical names. Copy a retained pattern with
`copy_pattern` under the active budget; release owned patterns with
`dispose_pattern` when discarding a parsed expression. The hook's default delegates to
`parse_predicate_leaf_in_scope`, preserving existing leaf parsing. For a dynamic
leaf argument such as `op(p | names | one)`, retain that value through
`PredicateArgument { argument, body }.bind_value(native_input_expression)`.
`SubjectScope` matches the same pattern and binds the whole unnamed subject. The
scoped body retains its native result kind through preparation. Nested scopes
bind `_` to their own subject and retain explicitly named outer captures.
The domain leaf checks its argument's existing string/scalar or native contract before
enumeration, including for empty selections. `PredicateScope` checks
a shared Boolean body. Both use the same value runner and frame restoration. Other leaves pass through `predicate_expression`, which receives a
`PredicateArgument`. Bind its subject with `PredicateArgument::bind`, using the
native input expression, and retain that expression in the host predicate leaf.
Explicit native headers also use `predicate_expression` for a mandatory pattern
check before the Boolean tree runs, even when a bare domain leaf uses none of
the header's names. This includes name and wildcard headers. A host that cannot
lower the shared check refuses the header. After admission, native short-circuit
and Truth behavior remain unchanged.
Shared predicates add no reflected native fields or new domain projections.

Expose every retained expression through `ProjectionHost::value_arguments` and
`scoped_value_arguments`. Declare its actual relative kind and separate lexical
captures. Preparation checks every branch, Boolean result, free name and effect
before enumeration. Execute selected bodies with
`NativeArguments::evaluate_predicate(expression, subject, captures, budget)`.
Pass `Some(clause_bindings)` for native match guards and `None` for ordinary
subjects. This preserves active functions, lexical captures and cumulative work;
creating another function request per candidate loses those guarantees.

| Predicate position | Subject passed to the shared body |
| --- | --- |
| Constructed `filter` | One constructed element |
| Native `filter` | One existing native subject |
| Native `root`, including diff roots | Each tested operation or ancestor candidate |
| `paths(..., to=...)` | One candidate native target |
| Native match `where` | The original incoming query value, possibly a collection; captures remain separate names |
| Diff `filter` | The existing change or side-owned operation subject |
| Scalar `filter_elements` | One decoded direct scalar element under the host's Data policy |

Ordinary native hosts can call `native_predicate_types` and
`evaluate_native_predicate` with the raw argument and actual subject. These use
shared projection/value preparation and execution without requiring functions.
For an ordinary domain leaf's dynamic value, use
`native_predicate_value_types` and `evaluate_native_predicate_value` with the raw
argument. Those helpers retain the same subject frame while the host checks its
existing string/scalar contract. A scalar host can instead bind its value and
use the ordinary value evaluator.

When an empty source still guarantees present primitive scalar elements, supply
`ValueType::ScalarKind` during preparation. It represents a Boolean, integer,
float or string with its exact kind unknown. Supply the actual primitive fact
when an element establishes it. Numeric, Boolean and string operands can defer
that exact kind; field, tuple and array access still reject a scalar. The marker
is preparation metadata and has no runtime value or declared type.

Shared predicate preparation checks required list, tuple and record headers
against established subject shapes before the host's `prepare_predicate` hook
and body preparation. Nested headers use the same check. Unknown shapes defer to
runtime matching. Value-match arms remain optional matches, so a structural arm
can fall through when its subject is scalar. These checks use the active budget.

Keep a scalar edit host's closed operator/effect policy and legacy comparator
or member leaves. Handles, diff sides, coordinates, source masks and validated
edit candidates remain host-owned. Existing native `Truth::Unknown` and Boolean
short-circuit rules remain unchanged; a shared value body requires a Boolean.

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
The frontend lowers ordered statements and defines emission policy. Zirql owns
program sequencing, binding scope and nested execution.
Comprehension-local names take precedence over native stage names in value
expressions. The head sees every binder; each source sees earlier binders, and
each filter also sees its current binder.

A caller holding a constructed prefix can call `read_value_suffix` with its
host, enclosing lexical names and cursor at the first outer pipe. The shared reader
wraps that value in the existing `Filter`, `Sort`, `Unique` and `Tally` nodes, keeping
stage locations and rejecting any stage after `tally`. Pass the active parsing
budget and the nesting allowance remaining for suffix wrappers. The shared
expression reader already owns both collection spellings and their generator
pipes. A host with known
constructed bindings implements `ValueReaderHost::is_constructed_binding` so
`rows | sort | unique` takes the same route as `(rows) | sort | unique`.
The default is conservative for native or unknown bindings. This fact does not
classify lexical locals or function results; their existing preparation can use
`lower_value_suffix` once their kinds are known. The helper builds the same
value nodes as the text reader. Native sources retain their host stage routing.
`is_common_value_call` identifies the fixed shared calls without deciding native
ordering argument shapes or declared function names.

`ValueReaderHost::field_pipeline_source` selects how an ordinary field prefix
such as `outer.items | filter(_ > 1)` continues:

- `FieldPipelineSource::Constructed` uses the common constructed suffixes.
  CSV and the word example use this route and need no native query evaluator.
- `FieldPipelineSource::ValueArgument` retains the shared `Field` value and
  passes it to `value_source` before reading the host's native stages. Choose
  this when the adapter executes that declared argument with the active
  prepared runner. Its existing kind checks decide which stages accept the
  selected value, including constructed values inside native reports.
- The default, `FieldPipelineSource::NativeCapture`, retains dotted-name
  native capture pipelines for direct native evaluation and standalone
  projections. `read_native_expression` always keeps native field postfixes.

The constructed and value-argument routes accept grouped and ungrouped fields
equally. This hook describes the field source's execution path. It does not
classify names or change `read_value_suffix`, which always reads constructed stages.

`ArgumentReader` retains original UTF-8 byte offsets. It has word, integer and
quoted-string helpers that skip whitespace, `#`/`%` line comments and nonnested
block comments. `word()` returns single-quoted names without their quotes while
retaining the complete byte range. Quoted strings retain literal comment markers. `remaining` and `advance` let a predicate callback read
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
admission, native record construction and emission. `copy_value` need not
copy an entire document. It must admit the new result representation before
allocating it. `construct_record` receives already-admitted field slots.

In `combine`, validate both operand kinds and every handle before calling
`combine_native_set`, including empty and discarded operands. Supply native
identities and candidates in the established result order: document order or
first appearance, for example. Zirql owns membership, combination and
deduplication. `NativeSetOperator::from_keyword` reads the three shared keywords.
Both operands already ran left to right from the same incoming selection.
The identity callback charges variable-size inspection; additional host scans
and owned string copies use the same budget.

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
[budget reference](runtime-limits.md) explains the accounting units.

## Constructed-value APIs

The value APIs have different jobs:

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

## Declared runtime contracts

Preflight checks what the expression says: its names, scope and known types.
Admission checks an actual value against its declared type before an operation
uses it. The two checks meet at the host boundary.

A trusted Rust operation can require a homogeneous sequence even though ordinary
Zirql arrays may contain different types. `DeclaredType<K>` expresses that
interface requirement. `ValueType` describes discovered or deferred facts,
including individual array positions; it is not an interface declaration.
`ValueType::Unestablished` means deferred knowledge, not a runtime absence value.

The runtime type API supports Boolean, integer, finite float, string, nominal
member, sequence, tuple, record, dictionary, set, keyed and native contracts.
`DeclaredType::Value` admits any constructed value; `DeclaredType::Dictionary`
checks key and value contracts, including nested constructed keys. Sequences and
keyed cells check every present element. Tuples have fixed positions. Records
require every nonoptional label and reject extra labels; optional fields may be
omitted. Checking a record preserves its stored field order. Duplicate
declaration labels fail. Scalar sets retain their strict element contracts,
including typed empty sets; `SetElementType::Value` admits general elements. Keyed rank, axis identity, sparse coverage and entry
order remain actual runtime facts. Nominal declarations use `NominalTypeId`, so
equal names do not make distinct types compatible. Integers do not implicitly
satisfy float contracts.

Use `admit_constructed` for borrowed constructed data:

```rust
use zirql::{DeclaredType, Location, admit_constructed};

let rows = DeclaredType::<()>::Sequence(Box::new(DeclaredType::Record(vec![
    ("name".into(), DeclaredType::String),
    ("count".into(), DeclaredType::Integer),
])));
admit_constructed(&value, &rows, &mut budget, Location::Query(10..30))?;
```

Admission discovers facts from the actual value and reuses the existing float,
label, nominal, set and keyed-shape validation. A heterogeneous array remains
accepted through existing value APIs and fails here only if the declared
contract excludes one of its elements.

Use `declare_constructed(value, declaration, budget, location)` when the
runtime value must retain its schema. It admits the actual value and returns a
`ConstructedValue::Declared` annotation with an owned `DeclaredType<()>`.
This subset cannot contain `Native(())`; native declarations remain with the
host. `ValueType::Declared` keeps this constraint beside the observed facts,
including `Unestablished` when only declaration metadata is available.

For a record with an optional field, the stored value can omit that field:

```rust
use zirql::{Budget, ConstructedValue, DeclaredType, Location, ProgramLimits,
            declare_constructed};

let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 1_000, nesting: 64, stream_items: 100,
});
let row = declare_constructed(
    ConstructedValue::Record(vec![("id".into(), ConstructedValue::Integer(4))]),
    DeclaredType::Record(vec![
        ("id".into(), DeclaredType::Integer),
        ("caption".into(), DeclaredType::Optional(Box::new(DeclaredType::String))),
    ]),
    &mut budget, Location::Query(0..20),
)?;
```

Pass `row` through ordinary `Bindings` and the value APIs. Access transfers the
selected field or element declaration; lookup of an omitted optional field
returns a typed absence. Re-admission checks the previous and new underlying
contracts, including when the actual value is absent. Repeated compatible
annotations replace the outer annotation instead of stacking wrappers.

Renderers borrow `value.data()` to inspect an annotation's underlying data,
and use `value.occurs()` for presence. Rendering must still walk only stored
record fields and keyed entries. It must preserve a stored absence separately
from an omitted field or cell. `into_data()` consumes an annotation at an
explicit data boundary. Removing it before further evaluation loses its schema.

Preflight uses existing `ValueQueryHost` facts and `ProjectionHost::scalar_types`
for native operands. Include `ValueType::Declared` when the adapter has a real
source declaration; observed record labels alone do not establish a schema.
Shallow fallback facts retain a container's kind while leaving its contents
uninspected. `ValueType::RecordKind` carries no labels or schema, and `TupleKind`
carries no positions or arity. Array element facts, keyed axes/cells and set
element identity may likewise remain unknown; these facts never claim that a
container is empty.
Supply metadata without projecting or scanning fallback data. Binding reads,
nominal-member resolution and native queries belong in the selected runtime
callbacks. An adapter that eagerly projects all free names before calling the
evaluator would defeat `default`'s lazy contract.

For native calls, implement the optional `RuntimeTypeHost<C>` alongside
`ProgramHost<C>`. It needs no projection implementation. Reuse the host's own
kind type as its `Kind`. `admit_native` receives the whole declaration, so a
native handle sequence can satisfy `Sequence(Native(Operation))` while its
handles retain their owner. The hook checks the actual wrapper and every member
against that declaration and the current context. For borrowed words, check source
pointers and ranges. For document handles, check owner and index, including
stale handles. A kind tag alone proves neither. If the host also wraps
`ConstructedValue`, expose it through the borrowed `constructed_value` hook so
the core checks language declarations. Native handles never become constructed
scalars or require serialization, downcasting or a shared registry.

`OperationSignature<K>` holds ordered parameter declarations and one result
declaration. Pass it with located arguments, the host, context, budget, call
range and an ordinary trusted Rust operation to `checked_invoke`. The call:

1. Checks arity and declaration structure.
2. Admits actual arguments in written order, at their own ranges.
3. Invokes the operation once with the same context and budget.
4. Admits the actual result at the call range before returning it.

For an already owned argument vector, an identity adapter can move its result:

```rust
let result = zirql::checked_invoke(
    &signature, arguments, &host, &mut context, &mut budget, call_range,
    |mut arguments: Vec<zirql::Located<HostValue>>, _, _| {
        Ok(arguments.remove(0).value)
    },
)?;
```

Here `HostValue` is the adapter's own value type and `signature` declares one
native parameter and the same native result kind. A borrowed argument slice is
also accepted; the operation receives that slice instead. Borrowing or moving
does not create another result representation. An operation producing new data
must charge work and admit result units before allocating it. Host failures keep
their concrete error type and source locations through `EvaluationError::Host`.
The host still checks operation purity and defines effects and emission policy.

Structural admission remains valid for that exact immutable value and unchanged
declaration. New external values and operation results require admission.
Native ownership and staleness depend on the current host context; nominal
resolution, actual key/field access and consumer shape policy still need their
runtime checks. The API stores no admission cache.
Each call performs its checks under the active work and nesting limits.

These declarations and calls are Rust library APIs. The common declaration
readers can also lower typed functions for a prepared request, as described
below. External `param` transport accepts `int`, `string` and `bool`. Report
preparation retains type facts and shared values for one invocation.

## Domain policy

The host defines its predicates, native identity, navigation and permitted
values. The [word model](examples.md) uses constructed strings and the shared
`starts_with` helper. A native word host can expose a `prefix` predicate over
borrowed text. A document host can retain handles and resolve their properties
from its owning context. The
[Zincite and Zirium integrations](semantic-models.md) show how those choices
affect queries, completeness and rendering.

The core can represent generic sparse keyed values. A consumer may support a
narrower subset. Validate that subset at the boundaries required by the
consumer; a final rendering check alone is insufficient when intermediate values
also must obey host shape rules. `evaluate_value_with_host_validation` lets a
trusted host check each successful runtime value before its parent uses it, with
the same budget and location. The [setup
guide](getting-started.md#use-zirql-in-another-rust-project) explains which
source version these APIs describe.

Model execution and source writes are separate host operations. The host checks
that each is valid for its current model and source.

## Library results and rendering

Use Zirql's [JSON rendering APIs](library-rendering.md) for constructed data
after evaluation. `to_json_value` produces an ordered JSON tree;
`render_json` returns compact or pretty buffered bytes. `render_json_value`
also writes existing JSON trees with host-owned document/completeness envelopes.
All three share the request budget and preserve supplied error locations.
These APIs belong to this working checkout, beyond published Zirql 0.2.0.
Native projection remains with the host. A shared bridge for mixed native and
constructed containers and consumer adoption remain separate integration work;
computational wrappers remain outside data rendering.

Return `NativeResult` or the host's value type to library callers. An emitter
can observe the value at a written position, but its buffering and transaction
rules are explicit host policy. A pure scalar expression must not emit or edit.
The frontend must also reject emitters or edits hidden inside opaque payloads in
pure bindings and nested constructor operands. Core validation cannot inspect
those payloads.

Specify whether written emission is terminal or pass-through, whether output is
buffered and what happens if a later operation fails. A CLI promising complete
output must buffer until the request succeeds. Library live edits and emitters
receive no automatic rollback from Zirql. Source-edit safety and transaction
rules belong to the host.

For signatures and budget charging helpers, use [the Rust API
reference](https://docs.rs/zirql/0.2.0/zirql/) or `cargo doc
--open` for your selected dependency. The [budget and diagnostic
reference](runtime-limits.md) explains how to translate errors and account for
host work.

## Correlated native projection

The word-model extension uses `project(w, {...})` to make report rows from
native occurrences. In Rust, `Projection<Q>` contains a located subject name and
ordered fields of `ValueExpression<Q>`. For native reports, `Q` is a normalized
`Expression` tree. Each subject is a singleton native selection.
`evaluate_projection` preflights every field before obtaining subjects, then
evaluates the fields in written order and returns an `Array` of `Record` values.
Input duplicates remain separate rows. Empty inputs still receive complete
scope, type and purity checks.

`ProjectionHost` extends `ProgramHost` with native kinds, stage applicability
and effect facts, visible nested selector operands, subject extraction and
scalar conversion. `native_expression_kind` walks bindings, grouping, sequences,
sets and native capture fields and refuses effects and emitters. Domain hooks
validate primitive arguments and retain native source failures. Handles require
an explicit domain projection before becoming scalar report data.

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

The entry points are `match_types(&matching, host, &input_kind, &scope, budget)`
and `evaluate_match(&matching, host, input, context, bindings, budget)`. Type
discovery returns ordered `NativeCapture<Kind>` fields. Execution returns
`Vec<Value>` rows built through `ProgramHost::construct_record`. Keep the schema
in the native collection wrapper, including when the row vector is empty.
Wrapping already-admitted rows moves them; admit any new host container before
allocation.

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

The entry points are `path_types(&paths, host, &input_kind, &scope, budget)` and
`evaluate_paths(&paths, host, input, context, bindings, budget)`. Type discovery
returns the ordered `NativeCapture<Kind>` schema for `source`, `sink` and
`steps`. Execution returns rows through `construct_record`, using `subjects`,
`subject_kind` and charged `copy_value` calls. Retain the schema when wrapping
an empty result, as for matching.

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

## Prepare native reports in one invocation

`evaluate_projection_with_types` and `evaluate_native_value_with_types` return
`ConstructedResult { value, types }`: an owned value and the facts prepared for
that expression. The existing evaluation functions return the value from this
same path. Hosts that need report schemas can consume this pair without running
a separate type-discovery walk.

Each call checks every written branch and resolves names and native operands
before obtaining projection subjects. Execution uses outer, subject and generator
slots. Preparation borrows that call's syntax and remains inside its current host
context; callers cannot retain an executable object and run it against another
document. New inputs and selected native results still receive runtime admission,
and hosts retain native identity, current ownership and source-coordinate checks.

`Bindings::with_local` shares its immutable outer map and retains only current
local entries. Constructed reads and generator values can share immutable owners.
`ValueQueryHost::read_shared_binding` and `ProjectionHost::share_constructed` let
hosts expose an existing `Arc<ConstructedValue>`; their defaults preserve owned
reads. The shared-local execution/lifting hooks likewise preserve existing host
implementations while allowing argument-ready owners to pass through unchanged.
Hosts must check the current context before returning a shared binding. These
hooks do not memoize domain operations.

The evaluator releases completed local frames and intermediate owners. Public
results, explicit snapshots, emissions and rendering retain their ownership and
copy-admission boundaries. See [request
accounting](runtime-limits.md#prepared-execution-and-sharing) for preparation,
sharing and copying charges.

### Embedded primitive arguments

A `ProjectionHost` can declare scalar/native argument bodies inside its stage
payload with `value_arguments(stage, budget, range)`. Each returned pair contains
a borrowed `ValueExpression<NativeExpression<C, H>>` and its relative input kind.
The core checks these bodies with the current lexical facts and prepares their
slots before enumerating subjects. This hook evaluates no argument data.

The default `evaluate_stage_with_arguments(stage, input, arguments, budget,
range)` forwards to `ProgramHost::evaluate_stage`. An adapter with computed
arguments overrides it and receives a borrowed `NativeArguments<C, H>` service.
`parts()` gives the current context and bindings. `evaluate(expression,
relative_input, budget)` runs one body declared by this exact stage and returns
`ConstructedRead`. The service retains its current host/context and cannot run
against a replacement context. It exists only for the active stage call.

Adapters invoke arguments in their existing written order and only when selected,
including inside a short-circuit predicate leaf. Argument bodies run through the
same scalar/native evaluator. Native operations execute afresh; the service does
not memoize their results. New selected data and operation results receive runtime
admission, and native owner/source checks remain with the adapter.

`ConstructedRead::value()` borrows the selected immutable data. A predicate can
inspect a string or collection without another deep copy. An adapter constructing
an owned native payload calls `into_owned(budget)`, which admits materialization
at the argument's original location. Body preparation is charged once per report
invocation; current argument frames, scope references, slot reads, runtime
admission and native work are charged when used. Current frames release their
owners when that native invocation ends. The lower-level `ValueQueryHost` bridge
hooks support this same path; ordinary value hosts keep their defaults.

## Prepare a function request

`FunctionHost` opts into named functions over the existing `ProjectionHost` and
`RuntimeTypeHost` contracts. Its `declaration_kind` hook maps complete signature
types to the host's preflight kinds. `is_operation_name` reserves registered
native operation names. The core separately reserves its builtins. Existing
hosts can continue using their current APIs without implementing these hooks.

`PreparedFunctions::prepare` registers every signature before checking bodies. Each
body can read its parameters and earlier immutable locals. The entry receives
explicit `Bindings`; functions receive that data through call arguments. Host
kinds used for native declarations must identify the complete native contract.
Native kind equality must agree with the host's declaration identity.

For ordinary sets and dictionaries containing native values, implement
`FunctionHost::{set_view, dictionary_view, construct_set, construct_dictionary}`.
Views expose retained values; constructors retain their owners without rendering.
`construct_native_set`, `construct_native_dictionary` and `tally_native_values`
provide hash-indexed collection construction, while `lookup_native_dictionary`
uses the same structural identity. The shared expression runner uses these
hooks for collections containing native values. Values from different query
paths or sources can coexist; normal context and stale-handle checks still apply.

`native_values_equal`, `native_value_hash` and `native_values_compare` recurse
through constructed data and host collection views. Native leaves use exact
`native_identity` tokens. `compare_native_values` orders same-kind leaves by
canonical identity by default; override it to order different native kinds.
A host without collection hooks reports an explicit unsupported operation.
Zirql renders constructed dictionaries as recursive key/value entry pairs,
preserving general keys. The host supplies native projections and emission
policy; a plain JSON object still requires string member names.

Native kind callbacks can use `pattern_function_value_kind` with owned pattern
declarations, or `pattern_function_value_kind_borrowed` with existing
`PatternFunctionDeclarationRef` headers. Both register signatures and discover
the selected expression's kind, including scoped native operands, without
preparing declaration bodies or copying their trees. Kind discovery does not
establish body purity. Before executing the request, separately prepare every
body with `PreparedFunctions::prepare_patterns` or `prepare_patterns_borrowed`,
including unused declarations.

Preparation resolves calls, slots and pure native expressions once for this
request. Native stages expose primitive argument bodies with `value_arguments`;
the same evaluator executes a selected body with the exact current context and
budget. `ConstructedRead::declared_type` retains an established function result
contract beside borrowed argument data; `into_owned` includes its annotation.
A native argument uses `NativeArguments::evaluate_native`, which returns a
`NativeRead` retaining the host owner and whole declared contract after the
callee frame ends. `value()` borrows the handle, `declared_type()` borrows its
metadata, and `into_shared()` transfers the retained owner. Native payloads never
pass through rendering. Constructed argument consumers retain their existing
`evaluate` path and require an explicit projection of native handles. Emitters,
effects and invalid nested selectors fail preflight even in unused or skipped code.

Runtime admission checks every fresh argument and result, including nested
collections and declared optional fields. An unchanged constructed owner can
retain admission for an equivalent contract within the request. Native admission
continues through `admit_value` in the current context, preserving owner and
stale-handle refusals. No `Clone`, `Send`, `Sync` or `'static` bound is added to
native values, and rendering is never used to pass them between calls.

Resolve all written entries with `prepare_value` before executing statements.
Each entry records its host kind and its owning unit; `evaluate` rejects an entry
from another unit before reading its resolved slots or call IDs. Native metadata
belongs to its prepared entry or function body, so sharing an immutable native
argument tree across lexical scopes cannot replace another body's kinds. The host keeps
the unit in one submitted request and passes the current context, bindings,
relative input and cumulative budget to each execution. These explicit borrows
let the ordinary statement runner use that same context between value calls.
`prepare` accepts borrowed declaration iterators, so typed-root composition can
share separate immutable function trees. It charges and retains references,
checks duplicate/conflicting names and resolves their signatures together.
The native argument bridge shares the program's outer binding map and overlays
only the active lexical locals. Prepared native indices look up their selected
binding directly, without rebuilding a vector for every caller binding per row.

For native record projections, prepare the fields once with their subject name
in scope and its singleton kind as relative input. Pass references to those
entries in written order to `PreparedFunctions::evaluate_projection`. It checks
unit and field association, duplicate labels, shadowing and scalar projection
before obtaining subjects, including for empty input. The ordinary and prepared
paths share subject binding, field order and row construction. Both return a
`ConstructedResult` with every admitted record alternative, so an empty report
retains its field schema. Hosts supply subject selection and scalar conversion;
field execution keeps the current context, relative subject and cumulative budget.

A projection inside a native value or function body uses the active argument
runner. Declare every field in `value_arguments`; `scoped_value_arguments` gives
each field the subject's local name and singleton relative kind while retaining
the incoming lexical scope. In `evaluate_stage_with_arguments`, call
`NativeArguments::evaluate_projection` and lift its value with its returned types.
The runner checks every written field before obtaining subjects and executes it
with current function parameters and locals. Nested projections use the same
runner and shared row construction; hosts do not prepare or enumerate fields
again during execution.

`FunctionRequest` preserves the one-shot context/budget-borrowing facade.
Normal return and failure release live slots and continuations. Use
`dispose_functions` for iterative cleanup of lowered native function trees,
including rejected declarations; hosts still own disposal inside opaque stages.

`ValueExpressionKind::Call` invokes a named pure function; `NativeValue` retains
a pure native result. A frontend with an exhaustive match must handle both
variants in traversal, lowering and disposal. `dispose_native_value` already
traverses them. The common reader accepts declarations, complete signature
types, calls and conditional expressions; see [function
embedding](embedding-programs.md#call-typed-pure-functions).

A frontend that retains native expression results opts into
`ValueReaderHost::retains_native_values`. This chooses `NativeValue` rather than
`NativeQuery` result representation independently of `field_pipeline_source`.
The `value_source` hook lowers an explicit function result, or a field selected
with `FieldPipelineSource::ValueArgument`, to a host stage with one declared
argument. Mark that stage with `ProjectionHost::is_explicit_source`;
it must ignore the runner's absent input placeholder and evaluate its declared
argument. This prevents a source such as `identity(op) | children` from reading
an ambient subject. Register the value in `value_arguments` and execute it with
`NativeArguments::evaluate_native` so native owners and constructed contracts
survive selection. Keep this path available in ordinary reports as well as
functions and predicates. Register pure native primitives through the same
argument runner and admission hooks.

Native `in` sources reuse `subjects` and `subject_kind`. Enumeration preserves
the host's sequence order, including explicit empty-sequence kinds. Each binder
retains its native owner; the eager shared continuation runner supplies candidate
order, filters, reducers and finite allowances. The host charges its enumeration
before allocation and checks current ownership. The core charges retained item
owners and grouped candidate expansion.

## Pattern adapter contracts

`PredicateArgument::argument` is `Option<Pattern>`. `ValueGenerator::In::binders`
is `Vec<Pattern>`, and `ValueGenerator::Bind::binder` is one `Pattern`.
`ValueGenerator::In::case` is `false` for strict generators and `true` for
selective generators. `Bind` has no selective form. A name uses
`PatternKind::Name(Located<String>)`; a discard uses `Wildcard`. Update builders,
free-name/scope walkers and iterative cleanup for the owned patterns. Preparation
and execution use their names in written order. `ValueQueryHost::prepare_predicate`
and `prepare_generator` receive pattern references. The latter's `element` flag
selects an `In` source element or the whole `Bind` initializer.

Native lists opt into `ProgramHost::positional_view`, which borrows only actual
positional storage. `ProjectionHost::positional_view_kind` supplies its admitted
element kind even when empty. Tuple and capture-record views keep their existing
hooks. A keyed array, tuple, set or opaque handle must not expose a positional
list view. Views retain their native owner and coordinates; matching selects only
the written prefix and never calls `FunctionHost::sequence_items` for a discarded
tail. Complete argument admission and current-context checks still belong to the
host's existing boundaries.

The ordinary `native_predicate_*` helpers need no `FunctionHost`. They share the
same pattern matcher over those native views. `ProjectionHost::pattern_constructed`
optionally borrows a native wrapper's existing constructed data, so nested scalar,
list and record patterns can inspect it without a projection copy. Exposed children
must retain coherent declared type evidence, including typed empty containers.
`_` remains the original subject, and native match captures stay separate bindings.
Inside an active function request, keep using its declared argument runner.

## Native computational values

A `FunctionHost` opts in with `prepare_computational`, which checks constructor
support in every written branch. It receives an explicit/contextual declaration,
or `None` when a queue must infer its element from selected actual input. The
default refuses computational constructors. Existing `declaration_kind` still
supplies ordinary host kinds when a whole declaration is known. Resolve named
constructor types with `resolve_type_name`.

Store `ComputationalValue<Self::Value, Self::Kind>` inside the adapter's value
wrapper. `computational_value` borrows it; `lift_computational` wraps an owned
result. The ordinary `copy_value` hook calls its budgeted `share(host, context,
budget, location)` method, retaining roots and metadata. It must not materialize
all stored items. No native `Clone`, `Send`, `Sync` or `'static` bound is needed.

`worklist_context` returns an exact opaque `NativeContextId([u64; 3])` token.
For example, Zirium stores the low and high halves of its 128-bit document owner
followed by its 64-bit revision: `[owner_low, owner_high, revision]`. Preserve
the full owner/snapshot identity, and change the token whenever native handles
can be invalidated. A host that cannot establish this immutable-snapshot guarantee
must decline the opt-in. Empty queues/sets retain this stamp too. Public
operations and runtime admission check the current stamp before retaining an
earlier invariant, including before the evaluator's reusable-admission path.

`native_identity` returns an exact `NativeIdentity([u64; 4])`. Its canonical token
must distinguish every semantically different owner, diff side and native object,
and agree with the host's same-operation/declaration comparison. Do not use a
printed name, address of a temporary wrapper, or raw hash as identity. The core
randomizes its hash and compares full tokens in collision leaves. Stored entries
also retain their native value, so identity does not discard handle ownership.

Call `admit_computational(value, expected, host, context, budget, location)` first
in `RuntimeTypeHost::admit_native`. A true result completes admission; false
leaves ordinary native admission with the adapter. The function runner uses this
same helper. It handles optional values and explicit tuple/record/sequence
containers, validates current context and established contracts, and delegates
fresh native leaves to the host. It never admits a stored queue by its tag alone.

Expose pop pairs through `construct_tuple` and `ProgramHost::tuple_view`, with
matching `tuple_view_kind` metadata. Native positional literals and scan snapshots
use `construct_sequence` and `sequence_items`; preserve their complete element
contract even when empty. Native sequence enumeration and `subject_kind` must
agree on their singleton element kind. Existing constructed literals retain
their constructed path. Explicit views can expose a constructed child without
converting other native fields or the remaining queue.

Keep computational wrappers out of scalar projection, JSON/data conversion and
emitters. Refuse them at their original query location, including nested
containers. Library results may retain them and continue with the public
budgeted operations in the same valid context. The host supplies any text syntax
for these operations.

## Correlated map construction

The common reader recognizes `map_by(key_query, value_query)` and calls
`ValueReaderHost::mapping` with a `NativeMap` of located native expressions.
A host can retain the expressions or lower its own syntax to the same
representation.

After preflighting both selectors, call `map_native_by` with admitted items, the
active runner/context and budget. Its key adapter returns `Some(strings)` or
`None` for another shape; the core enforces the single-string rule and duplicate
refusal. Its value adapter returns the native value to store. Both adapters use
the same singleton and existing saved/prepared-call scope. The core owns the
ordered loop and insertion-ordered entry storage. The host owns singleton copies, domain
admission and any explicit legacy per-entry conversion before insertion.

For native dictionary value sorting, implement `ValueReaderHost::map_sorting`.
The common reader calls it for `sort_by(value)`, including grouped selectors,
without asking the host to parse a `value` stage. Saved and local names keep
precedence. Other ordering selectors use `ordering` as before.

Call `sort_map_by_value(entries, budget, location, key)` with owned, admitted
unique-key entries. The callback projects each payload to `ScalarMapKey`:
`Null`, `Boolean(bool)`, `Integer(i64)`, `Unsigned(u64)`, `Float(f64)` or
`String(String)`. Reject unsupported nested payloads in that callback and admit
text copies before allocation. The core validates homogeneous scalar categories
and finite numbers, compares numbers without integer-to-float precision loss,
and keeps current-order ties. It reorders the owned entry vector; native owners
and coordinates stay intact.

Hosts using the constructed typed-key contract can instead move the same entry
vector through `order_native_by` with `NativeOrder::Sort`. Project each payload
to `NativeOrderKey::Value`: a present signed integer, finite float, exact string
or member of one coherent nominal type. This domain rejects Boolean and absent
keys and does not mix integer and float keys. Convert native counts with checked
signed conversion. The shared stable permutation retains entry payloads and
current-order ties without a separate dictionary result kind. `ScalarMapKey`
remains available for hosts whose scalar domain includes null, Boolean or mixed
numeric keys.

Both construction APIs return `Vec<(String, V)>`. Retain it as one dictionary
kind through bindings, copies and rendering. Call
`select_map(&mut entries, selection, budget, location)` with
`MapSelection::Reverse`, `Head(n)` or `Tail(n)` for dictionary bounds, whether
or not the dictionary has been sorted. Neither operation converts native values
or serializes JSON.

## Native existence

Implement `ValueReaderHost::where_exists` to retain the common reader's located
relation. The default disposes it and returns an unsupported-host error. Prepare
the pure relation against the singleton subject kind before inspecting input,
including empty input. Then call `filter_native_exists` with admitted original
rows and the active budget. Its callback evaluates the retained relation against
an admitted singleton using the current bindings, parameters and function locals.

Return an admitted native relation result. A nonempty result keeps the original
subject; a complete empty result rejects it. Unknown nonemptiness and relation
errors must refuse at their responsible locations. Keep original handles, order,
duplicates, owners and diff sides. Rendering and scalar truth are not cardinality
checks.

For required nested observations, override `Host::filter_truth` to admit the
completed shared Boolean result. Reject final `Unknown` with the host's
responsible diagnostic. Keep decisive conditions such as `unknown or true`
valid; an unknown leaf alone cannot determine the whole condition. The default
keeps ordinary best-effort filtering unchanged.

Zincite scopes complete-or-error observation to the nested body and restores it
on every result. Native results that omitted uncertain endpoints or final
unknown matches retain one responsible error through saved copies and native
collection operations, including empty results. Nested observation refuses that
value; unrelated report limitations do not establish incompleteness.

## Native fixed-point execution

Implement `ValueReaderHost::fixed_point` to retain the common reader's body; the
default disposes it and returns an unsupported error. Delegate replacement and
cycle handling to `evaluate_native_fixed_point`, passing one mutable context and
the active budget through body execution, checkpoint copying and comparison.
Use `ordered_native_equal` after checking kind and owner, including empty input.
Keep the body stable in prepared native arguments so functions retain their
parameters, locals and saved bindings on every iteration. Host callbacks retain
native admission, nested execution guards and approved effect policy.

## Fragments and computed object keys

Call `expand_fragments(parts, context, budget, location, resolve)` with the
host's parsed or borrowed fragments. The resolver returns `Cow<str>`: borrow
existing text and admit any owned temporary, such as decimal count text,
before allocation. The core resolves and appends each fragment in written
order. It does not parse inserted text, escape it or append a print newline.
Keep those rules in the host, and preflight every written binding and known
kind before effects.

Call `construct_string_entries(entries, context, budget, location, key, value,
duplicate)` to build computed string-key objects. The key callback can call
`expand_fragments`; the value callback can expand text or recursively construct
another object. Both use the same mutable context and budget. For each entry,
the core evaluates the key, refuses a duplicate, then evaluates the value.
The duplicate callback admits and constructs the host's existing diagnostic;
it cannot permit replacement. Entries retain insertion order.

Return admitted owned keys and payloads from the callbacks. They move into the
map, so a value can retain a borrowed native handle and its source coordinates
without serialization. A host choosing JSON supplies native snapshots and its
emission policy. Zirql's constructed renderer handles data admission, encoding
and escaping. Admit another result container before moving entries into it.

The constructor guards its one actual object level. A host with its own array
recursion can use `budget.within_evaluation(1, location, operation)` for each
array instead. Do not guard a shared object constructor twice. The guard
restores active depth on success or error and does not refund spent allowance.
Fragment expansion uses a zero-depth guard to bound resolver reentry.

Native ordering adapters can return `NativeOrderKey::Value` with an admitted
constructed scalar. Convert native counts to signed integers with checked
conversion when using this typed domain. Native handles need an explicit scalar
projection. The shared operation validates all keys before selecting results;
the adapter retains its input kind and owner when rebuilding an empty extremum.

`ProjectionHost::set_kind` admits set operands during shared preflight. Its
default requires equal kinds. Override it to reject kinds without a native
membership identity, including when an operand is empty or its branch is unused.
Runtime combination still validates every retained owner and handle.

## Selective generator adoption

The shared reader, callable evaluator and local CSV/word examples in this
checkout support `for case pattern in source` and `| case pattern in source`.
The [measurement example](values.md#select-matching-values) demonstrates selecting
present records from mixed input. This core support does not establish support
in an installed Zirium or Zincite frontend.

Each consumer needs a dependency revision containing `ValueGenerator::In::case`
and callable builders updated with `case: false` for existing strict clauses.
Builders that select matches set `case: true`; common-reader frontends receive
the syntax through that reader. A host-owned surface must lower its selective
form to the same clause. Native inputs use existing pattern views and admission
checks; consumers retain their own handle, source-coordinate and rendering rules.

A Zirium adoption could select report findings with a present source location.
A Zincite adoption could select present model-backed measurement records before
constructing a report. Those adapters must supply their domain data and adopt
the dependency before advertising these examples. This feature changes no
consumer frontend or Data edit policy and adds no symbolic CP operations.

## Sequence neighbourhood adoption

The development checkout's shared reader, callable evaluator and local
word/CSV examples support `windows(items, size)`, `runs_by(rows, "field")`
and their constructed pipeline suffixes. A Zirium report could project operation
names into positional records, then compare adjacent pairs, consecutive same-name
runs and global groups as in the [local report](examples.md#compare-consecutive-runs-with-global-groups).
This example does not establish support in an installed consumer frontend.

Each consumer needs a dependency revision containing the new AST and suffix
variants. Common-reader frontends receive the syntax through that reader;
frontends with their own call or suffix dispatch must lower to the shared nodes.
Update exhaustive AST/suffix visitors and callable builders. For Zirium, retain
native operation handles alongside projected names using the existing positional
and record views and list/record constructors. Keep native ownership, source
coordinates and output conversion with the adapter. Zincite can adopt the calls
for positional reports; Data edit admission remains its own policy. Consumer
repositories are unchanged by this core slice.

## Immutable record adoption

The shared reader, callable evaluator and local CSV/word hosts in this development
checkout support `record with {field: expression, ...}`. The
[capacity scenario](values.md#compare-capacity-scenarios) compares original and
expanded capacity records through the local CSV executable. Installed consumer
frontends have their own dependency and syntax support.

Zincite and Zirium need a dependency revision containing `ValueExpressionKind::With`
and updates to exhaustive AST visitors and callable builders. Common-reader
frontends receive the spelling through that reader. A host-owned reader must
lower its static field list to the same shared node and retain the surface ranges.
Native record adapters use their existing record value/kind views and constructors;
whole record and field contracts must survive that boundary.

A Zincite report can project a model-backed capacity into a record, create a
larger-capacity scenario and compare both. Model facts, MiniZinc type admission
and Data edit policy remain host responsibilities. Updating capacities stored as
keyed-array cells is deferred. That design still needs to choose missing-cell
behaviour and preservation of axes; `with` does not provide keyed updates.
Zirium can retain a native operation beside changed report fields using its
existing handle and source-coordinate rules. This core slice changes neither
consumer repository and adds no symbolic CP operation.

## Record alignment adoption

The shared reader, callable evaluator and local CSV/word hosts support
`align(left, right, by="field")` in this development checkout. The
[benchmark comparison](examples.md#compare-benchmark-tables) demonstrates matched,
removed and new cases with explicit guards before reading each side's timing.

Zincite and Zirium need a dependency revision containing
`ValueExpressionKind::Align`, updates to exhaustive AST visitors and callable
builders, and frontend lowering to that shared node. Common-reader frontends
receive the call grammar through that reader. Host-owned parsers must preserve
the literal selector's surface range. Native records need existing positional
and record value/kind views and list/record constructors. Their equality and
hashing must agree for admitted key values, retaining source and diff-side
identity. Optional before/after contracts must survive downstream projections,
including empty native sequences.

A Zincite report can compare two projected model summaries by an explicit
stable field, retaining declarations beside their measurements. Model meaning,
MiniZinc admission and Data edit safety stay with Zincite. A Zirium report can
compare two projected operation summaries by an explicit unique label while
retaining operation handles and source coordinates. Its native semantic diff
and cross-document identity policy remain host responsibilities. Neither use
infers native identity from matching display text. Both consumers must choose
and validate their keys and rendering of absent sides before exposing these
reports. This core slice changes no consumer repository and adds no symbolic CP
operation.
