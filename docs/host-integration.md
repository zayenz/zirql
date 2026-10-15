# Embedding in Rust

An application supplies data, exposes the operations its users need and handles
the result. Start with constructed bindings when arrays and records describe the
input. Add native items when queries need source identity, navigation or access
to an existing model. The adapter is the Rust code connecting that model to
Zirql's traits. A frontend reads query text and builds the expressions that the
evaluator runs.

## Dependency and toolchain

Add Zirql to a Rust crate with Cargo. The crate uses edition 2024 and requires
Rust 1.88 or newer. Development integrations pin a reviewed Git revision;
[set up the word model](getting-started.md#use-zirql-in-another-rust-project)
shows the dependency command and a complete executable. Generate API
documentation for that revision with `cargo doc --open`.

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

A host with its own value grammar can lower a completed prefix, then call
`read_value_suffix` with the cursor at the first outer pipe. The shared reader
wraps that value in the existing `Sort`, `Unique` and `Tally` nodes, keeping
stage locations and rejecting any stage after `tally`. Pass the active parsing
budget and the nesting allowance remaining for suffix wrappers. Zincite uses
this path after lowering a MiniZinc array or comprehension; the generator pipe
inside brackets remains part of its MiniZinc prefix. Saved constructed values
and function calls can use explicit grouping, such as `(rows) | sort | unique`.
Native name and call pipelines retain their host stage routing.

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
member, sequence, tuple, record, set, keyed and native contracts. Sequences and
keyed cells check every present element. Tuples have fixed positions. Records
require every nonoptional label and reject extra labels; optional fields may be
omitted. Checking a record preserves its stored field order. Duplicate
declaration labels fail. Sets retain their supported scalar element types,
including typed empty sets. Keyed rank, axis identity, sparse coverage and entry
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
The host still checks operation purity and defines effects and rendering.

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
reference](https://docs.rs/zirql) for your published version or `cargo doc
--open` for the pinned revision. The [budget and diagnostic
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
`ValueReaderHost::retains_native_values`. Its `value_source` hook lowers an
explicit function result at the start of a native pipeline to a host stage with
one declared argument. Mark that stage with `ProjectionHost::is_explicit_source`;
it must ignore the runner's absent input placeholder and evaluate its declared
argument. This prevents a source such as `identity(op) | children` from reading
an ambient subject. Register pure native primitives through the same argument
runner and admission hooks.

Native `in` sources reuse `subjects` and `subject_kind`. Enumeration preserves
the host's sequence order, including explicit empty-sequence kinds. Each binder
retains its native owner; the eager shared continuation runner supplies candidate
order, filters, reducers and finite allowances. The host charges its enumeration
before allocation and checks current ownership. The core charges retained item
owners and grouped candidate expansion.

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
ordered loop and lexical map storage. The host owns singleton copies, domain
admission and any explicit legacy per-entry conversion before insertion.

## Native fixed-point execution

Implement `ValueReaderHost::fixed_point` to retain the common reader's body; the
default disposes it and returns an unsupported error. Delegate replacement and
cycle handling to `evaluate_native_fixed_point`, passing one mutable context and
the active budget through body execution, checkpoint copying and comparison.
Use `ordered_native_equal` after checking kind and owner, including empty input.
Keep the body stable in prepared native arguments so functions retain their
parameters, locals and saved bindings on every iteration. Host callbacks retain
native admission, nested execution guards and approved effect policy.
