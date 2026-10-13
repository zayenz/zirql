# Values and comprehensions

A report can contain scalar data while its query retains native domain handles.
`ConstructedValue` holds the data part: Booleans, signed integers, finite floats, strings,
resolved nominal members, positional arrays, tuples, records, typed sets and
explicitly keyed arrays. A host handle becomes scalar data only through an
explicit host projection.

## Expression trees

`ValueExpression` is a located expression tree. It supports literals and names,
collection construction, checked scalar operations, access, inclusive integer
ranges, conditional branches and comprehensions. A host parser lowers its syntax
to this tree. The [getting started example](getting-started.md) evaluates one
without a frontend.

The [embedding guide](host-integration.md#constructed-value-apis) describes
validation, evaluation and copying APIs for this representation.

Use the same request budget across these operations. There is no deep `Clone`
implementation that bypasses copy accounting. Records preserve their written
labels and order; duplicate labels fail. Positional arrays and tuples use
positive one-based access positions.

## Checked arithmetic and conditions

Integers use `i64`. Overflow and division by zero fail. Integer division truncates
toward zero; remainder has the dividend's sign. Dividing or taking the remainder
of `i64::MIN` by `-1` also fails. Mixed integer/float operations require exact
integer-to-binary64 promotion. Strings receive no numeric coercion.

Floats must stay finite. Nonzero floating arithmetic rounded to zero fails.
Because an `f64` literal has lost its decimal spelling, a frontend must also
reject a nonzero decimal literal that parses to zero.

Boolean operations short-circuit, and conditionals evaluate only the selected
branch, with a required final else. Known incompatible types still fail validation
in skipped branches. A skipped missing access or zero division is not evaluated.
`ValueType::Unestablished` records missing type facts; it is not a runtime null or
an Unknown Boolean.

## Positional comprehensions

An array comprehension walks its generator clauses in order, evaluates each
source before introducing its own binders and evaluates the head after filters.
Later clauses can refer to earlier binders. Local binders are restored on success
and failure.

A word host can project `birch apple ash apple` to UTF-8 byte lengths
`[5, 5, 3, 5]`. A comprehension can filter out the short word and double the
remaining lengths. Written here as notation for the expression tree:

```text
[n * 2 for n in [5, 5, 3, 5] if n > 3]
→ [10, 10, 10]
```

The current Rust representation uses `ValueExpressionKind::Comprehension` with
`ValueConstructor::Array` or `Set` and `ValueGenerator` clauses. Grouped binders
form a Cartesian product. Source order determines array result order. Set
construction removes equal elements and uses type order. Supported scalar set
identities are Boolean, integer, float and resolved nominal member; strings are
not set elements in this implementation.

Every candidate is charged before filtering, including candidates a filter
rejects.

## Nominal members

A nominal member carries a host-supplied `NominalTypeId` and `NominalMemberId`,
retained type/member names and declaration order. Equality uses the IDs within
one immutable host context for the request. Equal names do not establish identity.
Member sets use declaration order. Contradictory metadata, mixed nominal types
and member arithmetic fail.

The host establishes these members and their identities. Zirql does not load
or execute a domain model to discover them.

## Keyed arrays

A keyed array retains typed, ordered axes and sparse cells. It is distinct from a
positional array or nested arrays. A rank-one array may use integer keys `2` and
`5`; it does not silently invent keys `3` and `4`. A higher-rank cell has one key
component per dimension.

Native `KeyedEntry` values must have unique full keys in row-major axis order,
with the rightmost dimension varying fastest. A lowered `KeyedArray` expression
can accept pairs in written order and normalize them. Supplied axes keep their
explicit order; inferred axes use numeric or member declaration order.

A keyed comprehension evaluates keys and heads after filters. Sparse output is
valid in the shared core. An empty result still needs typed axes. Its
`empty_axis_sources` can establish those from integer/member arrays or typed sets,
using the surrounding scope outside this comprehension's binders. Nonempty
results do not evaluate these fallbacks.

Array access checks key types and rank. `index_set` requires rank one and returns
the typed axis as a set in numeric or declaration order. A host may require
contiguous axes or complete rectangular coverage even though the core supports
sparse cells. See [semantic model integrations](semantic-models.md) for a
concrete example of that boundary.

## Array summaries and requirements

`ValueExpressionKind::Reduce` selects a closed `ValueReducer`: `Length`, `Sum`,
`Forall` or `Exists`. The operand is evaluated completely before reduction.
Positional arrays and keyed arrays are accepted; keyed summaries visit present
entries in stored order. `Length` counts present values, including sparse cells,
and accepts any element type. It creates no axes and does not flatten nested
arrays. `Sum` accepts integers and adds them in order with checked `i64`
arithmetic. `Forall` and `Exists` accept fixed Booleans and scan every element,
even after the result is decisive. Sets, tuples and strings are not array inputs.

| Reducer | Result | Empty array |
| --- | --- | --- |
| `Length` | Integer number of present values | `0` |
| `Sum` | Checked integer total | `0` |
| `Forall` | Boolean conjunction | `true` |
| `Exists` | Boolean disjunction | `false` |

A comprehension operand materializes every selected head before a reducer runs.
A later arithmetic failure therefore fails the request even when an earlier
Boolean would settle the result. Validation checks every possible known operand
and element type, including unchosen branches. Deferred types receive ordinary
runtime checks.

`ValueExpressionKind::Require { condition, value, message }` contains a
`Located<String>` literal message. Preflight visits both expression operands and
admits the message bytes. Runtime evaluates the condition as a fixed Boolean.
On true, it evaluates and returns the value; on false, it fails with the literal
message at the requirement's range. The false case skips the value's runtime
arithmetic, access, construction and host checks. It still refuses undefined
names, unsupported frontend forms and incompatible known types during preflight.
Host projection and actual binding-type discovery can still inspect or copy
referenced bindings before runtime.

These operations run in the existing five value APIs and
`evaluate_value_with_host_validation`. They return existing integer, Boolean or
child value kinds, so concrete type discovery and charged copying need no
separate summary result type. Hosts choose surface spellings and effect policy;
Zirql executes no model or solver.

## Native operands and extraction

`ValueExpression<Q>` defaults to ordinary constructed expressions with `Q = ()`.
A host can use `NativeQuery(Box<Q>)` for a normalized native operand.
`query_value_types`, `free_query_value_names` and `evaluate_query_value` use the
same analysis and runner as ordinary values. `ValueQueryHost` supplies declared
query facts, scalar binding reads and native execution with current generator
locals and the active budget. Runtime branch selection controls when a query
runs.

`One` accepts a positional array and moves out its sole child. Zero or multiple
children fail at the extraction range. The shared `take_one` helper also serves
native string extraction. `count_value` converts a `usize` count to an admitted
integer only when it fits `i64`; ordinary native counts retain their original
type. String projections remain arrays until explicitly extracted.

`value_expression_types` exposes inferred facts without execution. A projected
array's element schema remains known when it has no rows, so preflight can reject
a sum over known string fields on empty input. See
[native projection](host-integration.md#correlated-native-projection) for the
subject and conversion contract.
