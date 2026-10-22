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
| Constructed data programs | `ConstructedBinding`, `ConstructedProgram` | Values with declarations, binding names, optional current input and parameters |
| Constructed expression trees | `evaluate_value` | Located expressions and immutable constructed bindings |
| Ordered native stages | `Host` and `evaluate` | Native input, predicate leaves, identity and domain stages |
| Native programs and reports | `ProgramReaderHost`, `read_program`, `PreparedFunctions`, `PreparedProgram` | Native syntax/types, values, domain calls, admission and emission policy |
| Native composition without functions | `ProjectionHost`, `evaluate_native_expression` | Native views, guard leaves, identity, ownership and shape |

The [minimal constructed program](embedding-programs.md#constructed-data-programs)
needs no native host traits. The [word host](getting-started.md#supply-the-input)
binds one declared string sequence. The [CSV host](csv-host.md) loads declared
record sequences, selects sole or named input, supplies parameters and renders
a complete JSON or Markdown result. Its [numeric orders recipe](csv.md#calculate-with-numeric-cells)
uses typed parameters, a named function and explicit scalar conversions. Both
examples use the same facade for reading, preparation, scope and evaluation.
Lower-level `ValueReaderHost`, `ValueQueryHost` and `evaluate_query_value`
remain available for adapters that need expression-only integration.

Native programs use the [common reader](embedding-programs.md#read-text-programs),
[function preparation and request program](#prepare-a-function-request), and
the [domain hooks](#implement-domain-hooks) below. Core `ExpressionStep`
variants carry values, projections, filters, matches, ordering, maps, existence,
fixed points and paths. Zirql prepares and executes their bodies and scopes;
adapters expose domain leaves and native views. `Stage` carries domain primitive
and guard payloads, while `Emit` retains explicit host output policy. The smaller
`evaluate_program` API handles programs containing only opaque native stages.

These program, facade and composition APIs require this development checkout;
registry Zirql 0.2.0 predates them. Use `cargo doc --open` for the selected
checkout's callable signatures. The [published Rust API](https://docs.rs/zirql/0.2.0/zirql/)
describes the released version.

The [borrowed-word example](../README.md#a-borrowed-source-query) implements
`Host` for string slices, retaining their source through evaluation. Use native
integration when queries need that ownership, source coordinates or domain
navigation; [construct programs in Rust](embedding-programs.md) shows the builders.

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

## Define ordinary native functions

`FunctionHost::native_function(name, budget, range)` directly returns one
`NativeFunction`: its stable name, `OperationSignature` and implementation.
Use this for eager pure functions with explicit arguments. The shared reader
constructs ordinary `ValueExpressionKind::Call` nodes, and `PreparedFunctions`
resolves the definitions alongside declared functions. It checks arity, scope
and known argument types for every written call, including skipped branches.
Execution evaluates selected arguments in written order, then `checked_invoke`
admits their actual values and the result with the current context and budget.
The retained signature supplies result facts even for empty native sequences.

Charge signature allocations before constructing the definition. Implementations
retain native owners and source coordinates and keep their singleton, current
owner, stale-handle, completeness and domain checks. The definition receives no
ambient bindings or relative input. Special identifier grammar remains in
`parse_domain_call`; lazy predicates and effectful stages retain their hooks.

The [embedding example](embedding-programs.md#define-a-native-function) shows a
minimal definition and its ordinary read, prepare and evaluate sequence. A
stage parser that has already consumed a callee and `(` can use
`read_function_call_in_scope` to share argument parsing without reentering its
stage recognizer.

## Recognize shared syntax before choosing a host path

When a frontend retains native stages or legacy literals, Zirql supplies its
common routing decisions. Call `probe_ordering_call(reader, budget, nesting)`
before giving an overlapping ordering name to the native parser. The probe
leaves the reader unchanged. `sort(rows)` is a constructed call;
`sort_by(names)` is a native selector; `sort_by(rows, "score")` is a constructed
call. Only a comma at the argument's own delimiter level distinguishes the two
`_by` forms. Nested delimiters, quoted names, strings and supported comments
cannot supply that comma. Malformed input and exhausted work/result/nesting
allowances retain the original byte locations.

After applying native-stage precedence and binding facts, use
`starts_common_expression(reader, budget, nesting)` to select the shared value
reader. `has_named_value_postfix` identifies a grouped or bare name followed by
field access; `has_native_field_pipeline` recognizes capture fields continued
by a pipeline. The host chooses their native or constructed meaning from its
existing kind facts. `ValueSuffixKind::named` classifies constructed suffixes,
including `tally`. Read them with `read_value_suffix`, or map an
already parsed host payload to `ValueSuffix` and use `lower_value_suffix`.
Zincite retains that payload mapping for forward functions whose result kind
becomes known after their bodies have been read.

Before allocating a separate host token stream,
`has_common_expression_syntax(source, is_native_call)` gives an allocation-free
admission prediction. Supply only native calls that also require value parsing.
`has_collection_syntax` distinguishes computed collection syntax from plain
JSON-shaped literals. These predictions spend no request allowance; charge
known lexer storage before allocation, then use the active request budget for
checked lookahead and reading. Zirium keeps its established lightweight
allowance for legacy literals and text containing common words. Recognized
malformed common forms still receive the common parsing charges and diagnosis.
`read_program` owns function collection and common program assembly. Hosts
supply native expression and type reading, host-only statements and output
policy through the [program reader hooks](embedding-programs.md#read-text-programs).
Their emission/edit stopping boundaries and token cursor updates remain local;
these hooks do not reimplement the shared statement loop.

Name admission has separate contexts. `is_unquoted_binding_name` checks ASCII
identifier spelling. `is_common_keyword` identifies syntax words for an external
name policy, such as CSV table names; it does not prohibit every such word as a
lexical binding. `is_common_operation_name` identifies conflicts with named
function declarations and is also used by core function registration. Hosts
append native operation and input/document exclusions. Preserve quoted identity
and contextual names: a valid saved `if`, `report` or `lookup` binding does not
become invalid because the same word has a grammatical use elsewhere.

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
input, binding and sequence wrappers add no logical nesting. Core value and
projection steps expose their children to the shared walk; callbacks inspect
only opaque host stages.

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
a shared Boolean body. Both use the same value runner and frame restoration. Other
leaves pass through `predicate_expression`, which receives a
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
captures, with `ScopedNativeArgument::allow_effects` set to `false` for native
arguments. Only core representation bodies inherit an admitted entry policy.
Preparation checks every branch, Boolean result, free name and effect
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
subject and ordered fields. The common reader lowers values and `project`
directly to `ExpressionStep::Value` and `ExpressionStep::Projection`.
`ValueReaderHost::parse_stage` and its scoped form return a whole `ReadStep`,
normally `ExpressionStep::Stage` for a domain operation. The reader owns native
composition and Boolean wrappers; hosts need no value-source or projection
factories.
`read_program` owns the common statement loop, function collection, parameter
positions and pattern bindings. Implement `ProgramReaderHost` to supply native
expressions/types, host-only statements and implicit output. The smaller
`read_statement_prefix` helper remains available for deliberately limited readers.
See [text programs](embedding-programs.md#read-text-programs).
Comprehension-local names take precedence over native stage names in value
expressions. The head sees every binder; each source sees earlier binders, and
each filter also sees its current binder.

A caller holding a constructed prefix can call `read_value_suffix` with its
host, enclosing lexical names and cursor at the first outer pipe. The shared reader
wraps that value in the existing `Filter`, `Sort`, `Unique` and `Tally` nodes, keeping
stage locations. Preparation checks each continuation against the actual result
kind. Pass the active parsing
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
- `FieldPipelineSource::ValueArgument` retains the shared `Field` value in a
  core `Value` step before reading the host's native stages. Choose this for
  execution through the prepared expression runner. Its kind checks decide
  which stages accept the selected value, including constructed values inside
  native reports.
- The default, `FieldPipelineSource::NativeCapture`, retains dotted-name
  native capture pipelines for direct native evaluation and standalone
  projections. `read_native_expression` always keeps native field postfixes.

The constructed and value-argument routes accept grouped and ungrouped fields
equally. This hook describes the field source's execution path. It does not
classify names or change `read_value_suffix`, which always reads constructed stages.

`ArgumentReader` retains original UTF-8 byte offsets. It has word, integer and
quoted-string helpers that skip whitespace, `#`/`%` line comments and nonnested
block comments. `word()` returns single-quoted names without their quotes while
retaining the complete byte range. Quoted strings retain literal comment markers.
`remaining` and `advance` let a predicate callback read
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
Core value and projection steps bypass `evaluate_stage`. The optional
`transition` hook surrounds their execution with host value policy; its default
runs the supplied callback with the same optional input, context and budget.
Zincite uses it to require complete observations when needed and retain any
incoming incomplete-selection evidence on the result. The hook does not
dispatch common algorithms.

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

Core composition prepares native bodies and their argument scopes before
execution. That metadata and its copies use the same work allowance as the
query. A query close to a legacy work limit can therefore fail during
preparation even when its native traversal would fit; host defaults and the
meaning of a work unit remain unchanged. Type flow and scoped-argument
collection share one walk.

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
including typed empty sets; `SetElementType::Value` admits general elements. Keyed
rank, axis identity, sparse coverage and entry
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
For mixed reports, implement `RenderHost::view` to expose borrowed containers
and project native leaves. Zirql owns recursive traversal and ordinary JSON and
Markdown layout through the [mixed-report bridge](library-rendering.md#mixed-native-reports).
The host checks native owners, supplies leaf descriptors and document/completeness
envelopes, and refuses computational queues and identity sets. Rendering runs
after evaluation and never supplies an identity for a native value.

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

For signatures and budget charging helpers, run `cargo doc --open` for your
selected dependency. The [published Rust API](https://docs.rs/zirql/0.2.0/zirql/)
describes registry 0.2.0 and does not include the development APIs above. The [budget and diagnostic
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
scalar conversion. `expression_kind` defaults to `native_expression_kind` for
native-only hosts. Function hosts use their registered, borrowed declaration
headers for kind discovery without preparing or copying function bodies.
`record_kind` describes an explicitly constructed native capture; ordinary
value records use `ValueExpressionKind::Record`. Domain hooks validate native
arguments and retain source failures. Handles require an explicit domain
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

The common reader returns `ExpressionStep::Match`. Its ordered clauses retain
native query bodies in `NativeBody<Expression<...>>`, and Boolean leaves use the
host's `Stage` payload type. A native `Guard` variant keeps those leaves distinct
from executable domain stages. `ProjectionHost::validate_guard_leaf` admits
the leaf's input, scope and native arguments; `evaluate_guard_leaf` returns
`Truth`. The core owns Boolean composition and short-circuit evaluation.

Preparation checks each source and guard against the original match input and
only the captures introduced by earlier clauses. `from` enumerates singleton
subjects through `subject_kind` and `subjects`; `bind` requires exactly one.
A match whose sources supply explicit input can execute inside a function
without an ambient selection. The prepared runner retains active function
arguments, lexical bindings and native results throughout clause execution.

`capture_selection_kind` and `construct_captures` preserve the ordered field
schema, including an empty result. `ProgramHost::construct_record` builds each
native capture. Expose singleton fields through `capture_record` and
`capture_record_kind`; field reads use the host's charged `copy_value`.
Keep kind metadata immutable and shared so kind cloning does not copy native
objects. Operand lookup, SSA equality, model ownership and diff sides remain
domain facts.

Standalone callers can use `match_types` and `evaluate_match`; composed queries
use the core step through `PreparedFunctions` or `evaluate_native_expression`.
Hosts do not wrap the common match in an executable native stage or discover
its clause arguments themselves. Only opaque native leaf arguments use
`value_arguments` and `scoped_value_arguments`.

Release successful trees with `dispose_expression`, `dispose_match` or
`dispose_native_value`, as appropriate. These helpers visit shared composition
iteratively, including `NativeBody` storage. Hosts own cleanup inside opaque
stage, set-operator and emitter payloads. Native handles keep their source
coordinates until an explicit projection or rendering boundary.

## Native path adapter

The common reader returns `ExpressionStep::Paths`. `parse_path_edge` maps one
located identifier to an existing native primitive without consuming more
syntax. `ProjectionHost::validate_path_edge` admits its kind and scope; the core
also checks `is_pure`. Targets use the same `Stage` guard payloads and
`validate_guard_leaf` hooks as other native predicates.

`path_neighbors` supplies admitted singleton neighbors in direct edge order.
`same_path_identity` compares native identity, and `path_steps` builds an ordered
selection from admitted singletons. Preserve native owners, source or diff sides,
unresolved-edge limitations and stale-handle errors in these hooks. Printed
names do not establish identity.

The core owns breadth-first traversal, visited comparisons, witness choice and
reconstruction under the request budget. Target predicates use candidate
singleton input; they retain active lexical and function bindings. A final
Unknown fails at the target location through the host's `require_guard` policy.

`path_types` and `evaluate_paths` remain callable standalone APIs. Composed
queries use the same algorithm through the core step. `capture_selection_kind`
and `construct_captures` retain the `source`, `sink` and `steps` schema even for
an empty result. Release returned path trees with `dispose_paths`; opaque edge
and guard payloads retain their host disposal contract.

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

`PreparedFunctions` also accepts an empty declaration iterator. A host can use
one request path for ordinary expressions, projections and function calls:
prepare the declaration unit, call `prepare_expression` for each written
expression with its lexical kinds and actual optional relative kind, then call
`evaluate_expression` with current bindings, optional input, context and budget.
`PreparedExpression::kind()` supplies the next expression's kind before any
statement runs. Zirium uses this path for textual queries and typed report builders,
including programs without functions. Its native stages retain direct execution,
document ownership checks and their existing budget policy.

Keep native leaf argument bodies in their owning payload while preparation
borrows them. Core composition retains its own arguments and selects constructed
or native behavior from the admitted input kind. Report-only entries require
scalar data; native entries retain host kinds and handles until an explicit
projection.

`PreparedFunctions::prepare` registers every signature before checking bodies. Each
body can read its parameters and earlier immutable locals. The entry receives
explicit `Bindings`; functions receive that data through call arguments. Host
kinds used for native declarations must identify the complete native contract.
Native kind equality must agree with the host's declaration identity.

For ordinary sets and dictionaries containing native values, implement
`FunctionHost::{set_view, construct_set}` and
`ProjectionHost::{dictionary_view, construct_dictionary, restore_dictionary}`.
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

Resolve all written expressions before executing statements. Value-only callers
can use `prepare_value` and `evaluate`; a core expression containing values,
projections and native stages uses `prepare_expression` and `evaluate_expression`.
Each entry records its host kind and its owning unit; execution rejects an entry
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

For a core projection step, preparation obtains `subject_kind` and checks each
field once with the subject name in scope and its singleton kind as relative
input. Duplicate labels, shadowing and missing scalar projections fail before
subjects are enumerated, including for empty input. Execution binds each subject,
runs fields in written order and lifts the result with its prepared record
alternatives. Empty reports retain their field schemas. Nested projections use
the active argument runner and keep current function parameters and locals.
Hosts supply subjects, scalar conversion and value/kind lifting.

For a submitted program, pass the prepared functions and canonical lowered
`Program` to `PreparedProgram::prepare`, together with initial binding kinds,
optional current-input kind, function limits and the active budget. Then call
`evaluate` with the initial immutable bindings, admitted caller parameters,
optional current input and mutable context. Zirql retains prepared entries and
advances statement scope for `Parameter`, `Bind`, `BindPattern` and `BindFields`.
The existing statement driver owns input retention and emission sequencing.

Native relative groups use `ExpressionStep::Group` with `NativeBody`. They run
through the same argument machinery as selectors and retain the host result
shape. Entry groups inherit the surrounding effect allowance; function bodies
and pure selectors remain pure. `ProjectionHost::admit_emitter` checks native
output-kind restrictions during preparation, before any emission executes.
Hosts retain source syntax, domain reserved names, native ownership and effects.

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
The common reader represents an explicit function result or a field selected
with `FieldPipelineSource::ValueArgument` as a core `Value` step. Preparation
checks that value's own tree against the actual optional input. An `Input`
wrapper around the step neither requires nor grants ambient input: a top-level
expression can use its current input, while a function still receives only its
explicit parameters and locals. `NativeValue` keeps native owners and constructed
contracts through subsequent stages; `NativeQuery` retains scalar projection.

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

Ordinary native views also retain the [infix operator contract](library-operators.md#equality).
A native positional list holding `1, 2` can equal constructed `[1, 2]`, while
`[1, 2] == [9, true]` fails compatible-type admission in either representation.
Relations require present numeric operands; exposing a list view does not give
that list a structural infix order. Sorting uses its separate structural order.

The evaluator checks deferred operands only when their comparison executes.
Untaken conditional and Boolean branches retain their lazy behavior. Explicit
optional declarations and empty element schemas remain available during admission;
opaque native leaves keep the adapter's owner, source and diff-side identity.
Hosts need no comparison policy flag for their ordinary views.

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

## Native ordering and map construction

The common reader constructs `Ordering`, `Mapping` and `MapSorting` steps
directly. `NativeBody` retains each selector as a stable prepared argument;
selectors run against singleton subjects and keep native results even when the
same native kind also supports scalar projection. A host supplies no common
ordering or map factory.

`ProjectionHost::ordering_key_kind` checks declared key kinds and
`ordering_key` projects one native result to `NativeOrderKey`. The core owns
stable ordering, first or all tied extrema, and selector evaluation order.
`admit_ordering` checks domain admission before iteration. Empty native extrema
retain the original kind, element declaration, owner and source or diff side.
Kind, selector and effect preflight still runs on empty inputs.

For `map_by(key_query, value_query)`, `mapping_kind` declares the dictionary
kind, `mapping_keys` exposes the key strings, and `construct_map` wraps the
ordered entries. The core evaluates the key before the value for each subject,
requires exactly one string, and refuses duplicates before executing that
entry's value. Native payloads retain their owners and coordinates.

`map_sort_kind`, `map_entries` and `restore_map` admit and rebuild native
dictionaries. `sort_by(value)` uses the common value sort without treating
`value` as a domain stage; saved and local names keep precedence. Bare `sort`
orders dictionary keys. Both preserve dictionary kind and current-order ties.
Native ordering keys use the same structural contract as constructed keys.

Lower native `head`, `tail`, `reverse`, `check`, `count`, `unique` and `tally`
to `ExpressionStep::Collection` with the existing `Stage<()>` operation.
Computed bounds use `Head`, `Tail` or `Check` value steps. The native hooks admit
collection kinds and restore result shapes. Implement `collection_unique_tally`
by calling `evaluate_native_unique_tally` on the existing `FunctionHost`. That
helper uses runtime admission, structural hash buckets and exact native identities.
`ordinary_value` exposes only a scalar/literal wrapper's actual data while retaining
the original wrapper as representative. Borrowed dictionary views support common
selection and sorting; `restore_dictionary` preserves empty contracts. `Stage::Filter`
and `Stage::Host` are not collection tags. Native filter uses its direct
`ExpressionStep::Filter` and shared `Predicate<Stage>` tree.

The lower-level `map_native_by`, `order_native_by`, `sort_map_by_value` and
`select_map` algorithms remain available to standalone callers. A composed
query uses the core steps so preparation and execution share its active frame.

## Native existence

`ExpressionStep::WhereExists` retains one `NativeBody`. Preparation admits the
pure relation against the singleton subject kind, including empty input.
Execution uses the active prepared runner and `filter_native_exists`; a nonempty
relation keeps the original subject, while a complete empty relation rejects it.
Incoming order, duplicates and native ownership survive filtering.

`selection_kind` admits identity-bearing selections, `validate_existence`
checks relation compatibility, and `admit_selection` checks runtime completeness,
owner and shape. `selection_items` and `restore_selection` expose and rebuild
the native selection. `SelectionShape::Filter` allows hosts to distinguish a
filtered singleton from a preserved whole selection. Unknown nonemptiness and
relation errors fail at their responsible locations.

`filter_truth` controls the host's ordinary predicate completeness policy;
`require_guard` controls required match and path predicates. The core completes
Boolean composition before invoking these policies, so `unknown or true` can
remain decisive. Zincite retains responsible incomplete facts through saved
values and native collection operations, including empty results. Unrelated
report limitations do not establish that a relation is incomplete.

## Native fixed-point execution

`ExpressionStep::FixedPoint` prepares its `NativeBody` against the complete
current selection. This differs from existence's singleton-relative body.
The active runner preserves function arguments, lexical bindings and the
request budget on each iteration. The core uses `evaluate_native_fixed_point`
for replacement, convergence and cycle detection.

`fixed_point_kind` admits the body's result kind and returns the selection kind
for the next iteration. Its default requires exact input/result kind equality;
Zirium also retains its existing operation-stream to string-stream transition.
`same_selection` compares
ordered native identities after owner and kind checks, including empty input;
`admit_selection`, `selection_items` and `restore_selection` preserve runtime
completeness and shape. The body must return an admitted selection kind.
Native navigation and any domain-specific closure optimization stay with the
host. Body effect admission follows the surrounding host context. Zincite
requires pure bodies; Zirium entry programs preserve their pass-through
emission on every iteration, including the final stability iteration. Pure
functions and constructed contexts still reject effects.

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

Native ordering adapters return `NativeOrderKey::Value` for constructed data,
`Count` for a native unsigned count, `Text` for native text, or `Json` for an
existing JSON value. `sort_map_by_value` takes the same descriptors directly.
The borrowed comparison bridge retains exact unsigned leaves without changing
ordinary signed arithmetic or declarations. Hosts admit copied storage before
constructing descriptors; the shared operation admits all keys, including nested
nominal metadata, before comparison. Native handles need an explicit supported
projection. Restoration retains the original kind and owner for empty extrema.

`ProjectionHost::set_kind` admits set operands during shared preflight. Its
default requires equal kinds. Override it to reject kinds without a native
membership identity, including when an operand is empty or its branch is unused.
Runtime combination still validates every retained owner and handle.

## Selective generator adoption

The common reader lowers `for case pattern in source` and
`| case pattern in source` to `ValueGenerator::In { case: true, ... }`.
Strict clauses use `case: false`. Native inputs use the host's existing pattern
views and admission; source handles do not become positional data implicitly.
See [selective comprehensions](library-collections.md#comprehensions) and the
[measurement example](values.md#select-matching-values).

## Sequence neighbourhood adoption

[Windows and runs](library-grouping.md) consume positional report data. A native
adapter supplies positional type/value views and list/record constructors;
project operation names or other domain fields explicitly before grouping them.
The [local report](examples.md#compare-consecutive-runs-with-global-groups)
compares neighbouring pairs, consecutive runs and global groups.

## Immutable record adoption

[Record updates](library-operators.md#record-update) use existing record value/kind
views and constructors. Preserve whole-record and field declarations, retained
native children and their owners. `with` updates report fields; it does not
provide keyed-cell updates or source edits. The
[capacity scenario](values.md#compare-capacity-scenarios) compares original and
expanded records without changing input data.

## Record alignment adoption

[Alignment](library-grouping.md#align) uses positional and record views, matching
key equality/hash rules, and list/record constructors. Retain optional
before/after declarations through downstream projections, including empty native
sequences. Matching labels do not establish native identity or cross-document
ownership. The [benchmark comparison](examples.md#compare-benchmark-tables)
uses explicit guards before reading optional sides.

The development [Zincite](https://zayenz.se/zincite/guides/query-comprehension-contract/#transform-report-rows)
and [Zirium](https://zayenz.se/zirium/guides/query-language/#transform-positional-reports)
report frontends expose these common forms. Their guides identify the supported
dependency revisions; registry Zirql 0.2.0 and installed consumer releases retain
their documented support. Zincite keeps model facts, MiniZinc admission and Data
edit restrictions. Zirium keeps native MLIR/diff meaning, handle ownership and
source-edit policy. The shared evaluator and renderer add no solver integration.

## Declaration metadata and scalar arguments

`copy_declaration` copies Zirql-owned declaration structure with the request's
work, result and nesting limits. It retains record field order, nominal IDs,
native kind payloads and the element declaration of an empty sequence. It
charges each allocated child slot and copied field-name byte before allocation;
the root is returned inline. An adapter storing that root in `Rc` or `Arc`
charges its wrapper separately. Cloning a native kind still follows the host's
own storage and accounting rules. The Rust API includes a runnable example.

`contains_native` inspects this same structure and treats native kinds, queues
and identity sets as requiring native handling. It does not interpret native
kind payloads. Zincite deliberately classifies its native scalar/value wrappers
as constructed in its local admission logic; that classification cannot use
this structural test. Zirium uses the shared structural inspection directly.
Both development adapters use the shared copy operation while retaining their
`Rc` and `Arc` storage respectively.

`admit_type_facts` checks retained structured facts against a declaration,
including a dictionary's key and value contracts when it has no entries. It
uses the existing declaration rules; hosts still admit actual native kinds,
owners and values through `admit_value` and `RuntimeTypeHost::admit_native`.

For dictionaries exposed by `ProjectionHost::dictionary_view`, implement
`dictionary_view_kind` to borrow the retained key and value kinds, including
the schema of an empty dictionary. Call `admit_dictionary(value, expected, host, context, budget, location,
&string_key_kind)` from `RuntimeTypeHost::admit_native` after any computational
admission. A `true` result completes dictionary admission; `false` leaves other
native kinds to the host.

The helper checks nested schemas before entries, then admits each key before
its value. For example, a `dict<string, dict<string, int>>` declaration checks
both levels even when an inner dictionary is empty. The string-stream kind is
treated as a scalar string only in key position. Ordinary wrappers use constructed
admission; native leaves return to the host for owner, source and side checks.
A `Native` dictionary declaration requires the exact retained schema.

`admit_scalar_type` checks Boolean, integer, float or string facts through the
existing runtime type checker. `TypeFactPolicy::Strict` requires established
actual facts. `Deferred` permits `Unestablished` and `ScalarKind`; it still
checks any declared wrapper against the requested scalar contract. Fact
admission does not replace admission of the eventual value. Zincite defers
only its unknown argument facts, retaining its refusal of bare `ScalarKind`.
Zirium allows declaration-backed incomplete scalar facts and refuses bare
unknown facts in predicate and primitive positions. These choices remain
explicit in the adapters.

`nonnegative_integer_to_usize` converts an evaluated signed integer for counts
and indices, refusing negatives and platform overflow at the supplied location.
The host supplies the diagnostic message for its argument context. Domain kind
conversion, MiniZinc admission, MLIR ownership and native value admission remain
host responsibilities. These APIs require the development revision pinned by
the adapters; the published 0.2.0 crate does not contain them.
