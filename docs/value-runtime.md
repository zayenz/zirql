# Value evaluation in Rust

The [setup guide](getting-started.md) reads a word expression and evaluates it.
The reader returns a `ValueExpression<Q>` tree whose nodes retain query
locations. The evaluator checks scope and types, selects branches and builds the
result. `Q` is the type used for native queries inside that tree.

This page describes those Rust interfaces and their ownership rules. The [value
reference](values.md) explains the expressions themselves.

## Choose the evaluator

| Input | APIs | Adapter |
| --- | --- | --- |
| Constructed expression tree | `evaluate_value`, `validate_value_expression`, `free_value_names` | Immutable `Bindings<ConstructedValue>` |
| Shared reader output or native leaves | `evaluate_query_value`, `query_value_types`, `free_query_value_names` | `ValueQueryHost<Q>` |
| Native programs and projections | `evaluate_native_value`, `evaluate_projection` and their `*_with_types` forms | `ProjectionHost` |
| Calls to named pure functions | `FunctionRequest`, `PreparedFunctions` | `FunctionHost` |

`ValueExpression` defaults to `Q = ()`. With a native operand,
`ValueExpressionKind::NativeQuery(Box<Q>)` retains the normalized native tree.
`ValueQueryHost` supplies query type facts, binding reads and execution with
current generator locals and the active budget. Runtime branch selection
controls whether the native query executes.

The [word example](getting-started.md) uses the common reader and a
`ValueQueryHost` with constructed current input and the `words` binding. The
generic expression type still allows native operands, so the adapter explicitly refuses them. No native
program or function host is needed for that model.

The host supplies input facts through `current_input_types(range, budget)` and
an immutable value through `read_current_input(range, budget)`. The defaults
report unavailable constructed input. `CurrentInput` reads share one admitted
owner per evaluation invocation; sibling record fields each see that input.
Explicit sources do not read or replace it. Native `ExpressionKind::Input`
keeps its separate host-owned meaning. Pure functions receive collections as
explicit parameters and do not capture caller input.

The common reader lowers `filter(_.active == "yes")` to
`Filter { array: CurrentInput, predicate }`. `PredicateArgument<Q>` contains an
optional `Pattern` and a located value body. The matcher stages names in written
order and installs them together; a mismatch is a located error. The implicit
`_` retains the whole subject regardless of its argument pattern.
`PredicateSubject` reads the nearest predicate frame, independently of ordinary bindings. Filter preparation
uses the same Boolean/type/scope rules and the same iteration runner as a
comprehension. Typed-empty element facts survive selection.

`PredicateArgument::bind(subject)` creates a `PredicateScope` expression for one
actual subject. Scalar hosts can evaluate it with `evaluate_value` or
`evaluate_query_value`. Native adapters retain the bound expression as a declared
argument and use the active `NativeArguments::evaluate_predicate` runner.
For an existing domain leaf's non-Boolean argument, `bind_value(subject)` creates
`SubjectScope` and keeps the leaf's own argument type check.
`native_predicate_types` and `evaluate_native_predicate` support ordinary native
bodies without a `FunctionHost` or function session. The latter accepts the raw
predicate argument and actual host subject, retaining its native bindings.
The `native_predicate_value_types` and `evaluate_native_predicate_value` forms
serve an ordinary domain leaf's non-Boolean argument with the same scope.
`read_value_suffix` takes the host and enclosing lexical names so filters after
host-lowered prefixes retain the same name and call resolution.

## Values, facts and declarations

`ConstructedValue` holds absence, Booleans, signed integers, finite floats,
strings, nominal members, positional arrays, tuples, records, general
dictionaries, sets and keyed arrays. Native handles remain in the host's
representation and require explicit projections to become constructed values.

`constructed_value_type` discovers facts from a value. `ValueType` can retain
possible field schemas and element kinds even when a result is empty.
`Unestablished` means missing knowledge; it is neither null nor unknown truth.

`DeclaredType` expresses a required interface contract. Use `admit_constructed`
to check a borrowed value, or `declare_constructed` when the immutable value
must carry that declaration through further evaluation. Optional fields may be
omitted; required and extra labels are checked. Renderers inspect `data()` and
stored entries, preserving omission separately from stored absence.
See [runtime contracts](host-integration.md#declared-runtime-contracts) for native
admission and checked trusted calls.

## Validate before executing

`evaluate_value` validates actual binding types before execution. The explicit
validation and free-name APIs are useful before obtaining domain data, but do
not replace runtime checks. Keep validation, binding reads, copying and execution
on the same `Budget`.

All written branches receive scope, purity and known-type checks. Selected
runtime branches perform data access and host execution. For a binding used
only in a default fallback, supply shallow kind or declaration facts without
projecting or scanning the fallback's data. Eagerly loading every free binding
would break the lazy-default contract.

`evaluate_value_with_host_validation` invokes a trusted value-policy callback
on each successfully executed value before its parent consumes it. The callback
shares the budget and can enforce domain restrictions such as rectangular keyed
coverage. Unselected runtime branches do not call it; structural and known-type
preflight still covers those branches.

## Map operations to nodes

| Language form | `ValueExpressionKind` representation |
| --- | --- |
| Scalar and collection construction | `Literal`, `Array`, `Tuple`, `Record`, `Dictionary`, `Set` |
| Scalar operations and exact string helpers | `Unary`, `Binary` with the corresponding operator |
| Field/position access and optional lookup | `Field`, `ArrayAccess`, `TupleAccess`, `Lookup`, `HasKey` |
| Absence checks and fallback | `Occurs`, `Absent`, `Default` |
| Conditional selection | `Conditional` |
| Computed dictionary | `Dictionary` entry pairs or `DictionaryComprehension { clauses, key, head }` |
| Array or set comprehension | `Comprehension` with `ValueConstructor` and `ValueGenerator` clauses |
| Scoped collection selection | `Filter { array, predicate }` |
| Bounds and movement | `Head { collection, count }`, `Tail { collection, count }`, `Reverse` |
| Cardinality assertion | `Check { collection, operand, message }` |
| One bound predicate | `PredicateScope { subject, predicate }`, `PredicateSubject` |
| Summaries | `Reduce` with `ValueReducer::{Length, Sum, Forall, Exists}` |
| Requirement | `Require { condition, value, message }` |
| Grouping | `GroupBy { rows, field }` |
| Ordering | `Sort { array, field, descending }`, `Extremum { operator, array, field }` |
| Keyed inspection and pairing | `Entries`, `ZipExact { left, right }` |
| Exact-one extraction | `One` |

Selectors and requirement messages retain their original locations. Grouping
uses a static `Located<String>` field. Sorting and extrema have an optional
static field; no field selects the element itself. Keys use structural value
order, including mixed kinds, absence and compounds. Invalid selector shapes fail
preflight, while missing fields may remain deferred until runtime. Empty report
projections retain their declared row facts; an empty literal invents no schema.

`One` moves the only child of a positional array. `take_one` supplies the same
cardinality check for native extraction. `count_value` converts a native
`usize` count to an admitted signed integer only when it fits `i64`.

`Record` retains static quoted field schemas. `Dictionary` and
`DictionaryComprehension` evaluate constructed key expressions. All-string
entries produce `ConstructedValue::Record`; other entries produce
`ConstructedValue::Dictionary(Dictionary)`. Convert owned key/value entry pairs
with `entries.into()`; the storage exposes their insertion order. Prepared
continuations check each key and structural collision before running
its value, including across generator candidates. Empty `{}` produces a record.
General dictionaries retain key/value type facts and support
`DeclaredType::Dictionary`; `DeclaredType::Value` admits any constructed value. General sets use
`SetElementType::Value`, while homogeneous scalar sets retain their precise type.

`ArrayAccess` also accepts dictionaries and constructed keys. It is strict: a
missing stored key fails, while stored absence succeeds. `Lookup` remains optional.
Both preserve declared child contracts; record schemas keep their string labels.

`constructed_value_hash` and `compare_constructed_values` expose budgeted
structural hashing and ordering for every constructed kind. They validate input,
keep distinct kinds separate and treat floating signed zeros equally. Lists and
tuples retain positional order. Sets and dictionaries use canonical membership
and associations, so insertion order does not affect hashes or comparison.
Hashes are for in-memory indexes, not persistent identifiers. Set construction
and tally use hash collision buckets with structural equality checks. General
dictionary storage builds its lookup index lazily and clears it on mutable access.

## Keyed construction

A nominal member carries `NominalTypeId`, `NominalMemberId`, names and declaration
order from one immutable host context. Contradictory metadata and mixed nominal
types fail. Equal printed names cannot replace those identities.

Native `KeyedEntry` values require unique full keys in row-major order.
`KeyedArray` expression pairs may arrive in written order; construction
normalizes them. Inferred row and column keys use numeric or member declaration
order. Explicit ordered domains are represented as `KeyAxis` values.

Both `KeyedArray` and `KeyedComprehension` accept located
`declared_axis_sources: Option<Vec<ValueExpression<Q>>>`. Each source runs once,
left to right, in surrounding scope before literal entries or generators.
Integer/member arrays and typed sets establish complete domains, including
unused keys and typed empties. The result retains those domains, their order
and native nominal identities. A literal's concrete `axes` and declared sources
are mutually exclusive.

`KeyedComprehension::empty_axis_sources` is a separate lazy fallback for zero
emitted cells. Nonempty output does not execute it. It resolves outside the
comprehension's binders too, but cannot coexist with declared sources. Existing
host lowerings can retain that fallback without changing its timing.

The common reader lowers `[key: value]`, tuple keys and indexed comprehensions
to these nodes. Contextual `axes(source, ...)` follows the closing bracket,
before access or pipelines. `[]` remains positional; `[] axes(...)` constructs
an empty keyed value. See [indexed examples](values.md#keyed-arrays).

`index_set(a)` extracts the ordered typed keys of a rank-one array. `entries(a)`
exposes present cells, and `zip_exact(a, b)` compares ordered domains and full
keys at runtime, including nominal metadata. Neither helper bypasses the host's
shape policy. Declared-source evaluation, conversion and retained-domain copies
use the active work, result and nesting allowances.

## Ordering, grouping and ownership

Sorting extracts each key once and uses a fallible stable merge. It uses
O(n log n) comparisons and O(n) temporary descriptors and positions. An owned
array moves its values by permutation; a shared input needs an admitted owned
output copy. Extrema instead scan all keys and retain one winner, materializing
only the selected value at an owned-result boundary.

Grouping evaluates its rows once. It copies new group keys and labels, then
moves existing rows into the groups. Type discovery retains possible original
row schemas without predicting which keys will occur. Keyed entries and exact
pairing also move admitted cells into their new containers where ownership
allows it.

Alignment evaluates both positional record sources once, left then right.
A hash index uses the host's structural hashing and equality to reject duplicate
keys on each side and match equal keys across sides. Staged rows keep the left
key and original before/after values, then append right-only rows in their input
order. Existing record and list constructors retain native handles and source
coordinates. Missing sides become ordinary absence. Traversal, index buckets,
collision comparisons, staging and output use the active budget.

Type discovery retains the result's `key`/`before`/`after` record shape and each
side's optional record contract, including declared empty native sequences.
An untyped empty array supplies no inferred row schema. The [callable
interface](embedding-programs.md#record-alignment-expressions) uses the same
preparation and execution as the shared reader.

New containers, metadata, comparisons and copies spend the continuing budget.
See [resource accounting](runtime-limits.md#what-uses-the-allowance) for the exact
boundary, including producer admission before allocation.

## Immutable reads and owned results

Evaluation prepares local names once and reads them through resolved slots.
Repeated reads can share unchanged strings, collections and selected fields.
Reducers inspect shared input directly after eager operand evaluation; they still
scan every required element. Generator sources retain current immutable owners,
and completed local frames release their temporary references.

Returned `ConstructedValue` results remain owned. Separate result fields and
explicit snapshots receive admitted materialization, so changing an owned Rust
result cannot change an earlier result or binding. Fresh values still receive
runtime checks. Sharing does not scan or execute a default's unselected fallback;
all written branches keep their structural and known-type preflight.

## Native computational collections

A worklist computation can keep pending items in a queue and visited native
identities in a set. The Rust function API represents their contracts with
`DeclaredType::Queue(Box<DeclaredType<K>>)` and `DeclaredType::IdentitySet(K)`.
A queue carries its whole element contract; an identity set accepts one native
kind. Constructed sets retain insertion order and structural membership
equality; sorting supports mixed and nested elements through structural order.
They remain separate from computational native identity sets.

| Value expression | Result |
| --- | --- |
| `Queue { items, element }` | A FIFO in positional input order |
| `PopFront(queue)` | `opt tuple<T, queue<T>>`; empty is absent |
| `PushBackAll { queue, items }` | Existing queue followed by every input item |
| `IdentitySet { element }` | Empty set of the explicit native type |
| `Insert { set, item }` | Set containing that identity; duplicates are unchanged |
| `Binary { operator: In, left: item, right: set }` | Boolean native identity membership |

Constructor type arguments use located `SignatureType` trees. An explicit
`element` supports empty queues. Function argument/result contracts and established
loop/accumulator state also supply queue element context. Nonempty input can
establish a homogeneous element declaration. An empty input without either an
explicit or contextual declaration fails. Identity-set elements must resolve to
native types; scalar and nominal set declarations are separate types.

Native handles retain their owner, side and source coordinates. Every fresh
queue element, appended item and identity-set operand receives whole admission.
A host context stamp identifies the immutable snapshot in which that admission
remains valid. Reusing an unchanged queue or set checks this stamp and its small
type metadata, without scanning stored elements. A stale or foreign context
fails at the active expression. Printed names do not establish identity.

A successful pop is an ordinary native tuple. Existing tuple patterns and
optional matching extract its fields; there is no new presence state. Empty
pop uses the existing absent value. Native/mixed `Array` and `Tuple` expressions
use explicit host container hooks, preserving the same native values.

`NativeQueue`, `NativeIdentitySet` and `ComputationalValue` are public native
library values. Their budgeted operations preserve retained aliases, and
returned wrappers remain usable after the function frame ends. They are
computational values for use during evaluation. Direct scalar projection,
rendering and serialization must fail with a located error. Identity-set
iteration and general set algebra are absent. The common expression reader accepts
`queue(items)`, `queue<T>(items)`,
`pop_front(q)`, `push_back_all(q, items)`, `identity_set<T>()`, `insert(seen, item)`
and native membership. The host resolves native type names and supplies identity
and container hooks. See [functions and control
flow](functions-and-control-flow.md#other-control-flow-forms).

## Selection contracts

`Filter`, `Head`, `Tail`, `Reverse` and `Check` run in the same prepared value
runner as other expressions. Bounds and check messages are expressions, evaluated
in written order. Only `Length` broadens reducer admission to sets/dictionaries;
`Sum`, `Forall` and `Exists` still require their existing arrays.

Adapters can reuse `collection_subject_types` for element/value predicate facts,
`collection_selection_types` for result facts, and
`restore_collection_declaration` when rebuilding retained rows. Preserve explicit
sequence/array element and dictionary key/value declarations before splitting
containers. Removing record fields weakens their presence to optional; moving
entries retains required fields. Do not copy stale positional facts into a
reordered result or infer an empty schema from retained rows.
