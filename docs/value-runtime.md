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

The common reader lowers `filter(starts_with("a")) | count` to
`FilterStartsWith { array: CurrentInput, prefix: "a" }` inside
`Reduce { operator: Length, array: ... }`. Typed callers use the same nodes and
`evaluate_query_value`, with the host's input facts and the same budget.
`read_value_suffix` also accepts filter/count after an already lowered value.

## Values, facts and declarations

`ConstructedValue` holds absence, Booleans, signed integers, finite floats,
strings, nominal members, positional arrays, tuples, records, typed sets and
keyed arrays. Native handles remain in the host's representation and require
explicit projections to become constructed values.

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
| Scalar and collection construction | `Literal`, `Array`, `Tuple`, `Record`, `Set` |
| Scalar operations and exact string helpers | `Unary`, `Binary` with the corresponding operator |
| Field/position access and optional lookup | `Field`, `Access`, `Lookup`, `HasKey` |
| Absence checks and fallback | `Occurs`, `Absent`, `Default` |
| Conditional selection | `Conditional` |
| Array or set comprehension | `Comprehension` with `ValueConstructor` and `ValueGenerator` clauses |
| Prefix selection | `FilterStartsWith { array, prefix }` |
| Summaries | `Reduce` with `ValueReducer::{Length, Sum, Forall, Exists}` |
| Requirement | `Require { condition, value, message }` |
| Grouping | `GroupBy { rows, field }` |
| Ordering | `Sort { array, field, descending }`, `Extremum { operator, array, field }` |
| Keyed inspection and pairing | `Entries`, `ZipExact { left, right }` |
| Exact-one extraction | `One` |

Selectors and requirement messages retain their original locations. Grouping
uses a static `Located<String>` field. Sorting and extrema have an optional
static field; no field means scalar input. Known incompatible kinds fail
preflight, while missing fields may remain deferred until runtime. Empty report
projections retain their declared row facts; an empty literal invents no schema.

`One` moves the only child of a positional array. `take_one` supplies the same
cardinality check for native extraction. `count_value` converts a native
`usize` count to an admitted signed integer only when it fits `i64`.

## Keyed construction

A nominal member carries `NominalTypeId`, `NominalMemberId`, names and declaration
order from one immutable host context. Contradictory metadata and mixed nominal
types fail. Equal printed names cannot replace those identities.

Native `KeyedEntry` values require unique full keys in row-major axis order.
A lowered `KeyedArray` expression can normalize pairs supplied in written order.
Explicit axes retain their order; inferred axes use numeric or member declaration
order. A keyed comprehension evaluates keys and heads only after its filters.

An empty keyed comprehension still needs typed axes. Its `empty_axis_sources`
can obtain them from integer/member arrays or typed sets, evaluated outside the
comprehension's local binders. Nonempty output does not evaluate those fallbacks.
Rank-one index-set extraction returns the typed axis as an ordered set.

The common value reader accepts `index_set(a)`, `entries(a)` and
`zip_exact(a, b)` for supplied keyed bindings, and `tuple.1` for one-based tuple
access. Keyed literal construction needs host lowering. Pairing compares ordered axes and full keys at runtime, including
nominal metadata. Each executed operand still receives the host value-policy
check. Neither helper bypasses domain shape restrictions.

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
kind. Scalar and nominal sets retain insertion order and support explicit
sorting; they remain separate from native identity sets.

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
iteration and general set algebra are absent. These operations have Rust
representations. A host supplies their text syntax; the common reader does not
parse worklist expressions.
