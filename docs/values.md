# Values and comprehensions

A report can contain scalar data while its query retains native domain handles.
`ConstructedValue` holds the data part: absence, Booleans, signed integers, finite floats, strings,
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

## Absence, lookup and defaults

Absence is one runtime value, distinct from an unknown fact and an error. The
shared reader accepts `null` and `<>`; Zirium uses `null`, and Zincite uses `<>`.
`occurs(x)` is true for every present value, including false, zero, an empty
string and an empty collection. `absent(x)` returns the opposite answer.

Use `lookup(value, key)` when missing data is expected. A record key is a string
label, a positional array key is a one-based integer, and a keyed-array key is
its existing integer/member scalar or tuple. A valid missing label, position or
sparse cell returns absence. `has_key(value, key)` distinguishes that miss from
a stored field or cell containing absence:

```text
has_key({"caption": null}, "caption") → true
has_key({}, "caption")                → false
lookup({"caption": null}, "caption")  → null
lookup({}, "caption")                → null
(lookup({}, "caption") default "untitled") → "untitled"
```

Wrong container types, key types, keyed ranks and nominal identities are errors.
A label outside an explicitly declared record schema is also an error. Ordinary
field and bracket access remain strict: they fail when the field or cell is
missing. Lookup keeps sparse coordinates and does not fill omitted cells.

`x default y` evaluates `x` once. It evaluates `y` only when `x` is absent.
A present false, zero or empty value keeps its meaning. A selected arithmetic,
host or resource failure propagates; default does not recover from errors.
All written branches still receive syntax, scope, purity and known-type checks.
The shared reader follows MiniZinc's left associativity and tight precedence for
`default`; parentheses make its boundary clear in arithmetic expressions.

Arithmetic, ordering and Boolean conditions require present values. Supply an
explicit default before using a possibly absent operand. Optional equality
makes two absences equal and an absence unequal to a present value, while still
requiring compatible known underlying types. An absent integer declaration
cannot compare with or default to an absent string declaration.

Rust hosts can attach `DeclaredType::Optional` and record schemas with
`declare_constructed`. Optional declared record fields may be omitted; required
fields must be present and extra labels fail admission. The immutable value
retains that declaration through bindings, copies, generator locals, access and
selected defaults. Stored absence remains stored, and omitted data remains
omitted. Unannotated heterogeneous values remain supported. See
[constructed declarations](host-integration.md#declared-runtime-contracts).

These forms are available in the development Git revision. Published alpha.5
predates them. Optional reports do not expand external parameters beyond
`int`, `string` and `bool`, or authorize optional MiniZinc data-edit targets.
User function signatures and optional parameter transport remain deferred.

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

## Keyed entries and exact pairing

`entries(a)` exposes a keyed array's present cells as a positional array of
records with `key` followed by `value`, in stored entry order. Rank-one keys are
integer or nominal scalars; higher-rank keys are tuples in dimension order.
The source binding retains its typed axes. Empty input returns an ordinary
empty positional array.

`zip_exact(a, b)` accepts two keyed arrays with identical ordered axes and
present full keys, including identical sparse coverage. Its result retains the
left axes and keys, with each cell containing a two-element tuple: the left
value followed by the right value. Component value types may differ. Equal
cell counts alone do not establish alignment. Different axis order, rank,
integer keys, nominal identity or present-key coverage causes a refusal.
Equal empty arrays retain their complete typed axes.

For two rank-two arrays with matching coordinates, a host can read their pairs:

```text
entries(zip_exact(before, after))
→ [{key: (1, 1), value: (3, 4)},
   {key: (1, 2), value: (4, 6)},
   {key: (2, 1), value: (5, 8)},
   {key: (2, 2), value: (6, 10)}]
```

The callable forms are `ValueExpressionKind::Entries` and `ZipExact { left,
right }`. The common reader accepts the exact one- and two-argument calls with
keyed bindings; hosts supply their own keyed literals and tuple-access syntax.
Shared validation checks known input kinds and axis rank/types in every branch.
Runtime compares the ordered axes and full keys, including retained nominal
metadata, at the call range. Skipped runtime alignment or missing-access errors
remain skipped; child failures keep their own locations.

Both operations use the ordinary value and native-value APIs. Entries moves
admitted keys and cells into records; pairing moves left axes, keys and both
cell values into tuple cells. New containers, slots, labels, comparisons and
depth checks spend the continuing budget. Later copies and rendering spend
that same allowance. Host checks still inspect each executed operand before a
parent consumes it, so entries cannot bypass a host's rectangularity policy.
Neither helper fills, truncates, reindexes or writes data.

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

## Stable record groups

`group_by(rows, "field")` groups a positional array of records by one literal
string selector. Its keys must all be integers, strings or members of one
nominal type. Keys keep their types; nominal equality uses type/member IDs.
Keyed arrays require an explicit conversion to positional rows first.

The result is an array of records with `key` followed by `items`. Groups follow
the first occurrence of each key. Each `items` array retains input order and
duplicates, including records whose other fields differ. Empty input returns
an empty array.

```text
group_by([
  {"machine": "b", "demand": 3},
  {"machine": "a", "demand": 5},
  {"machine": "b", "demand": 7},
  {"machine": "b", "demand": 3}
], "machine")
→ [{"key": "b", "items": [first row, third row, fourth row]},
   {"key": "a", "items": [second row]}]
```

A comprehension can read `g.key` and reduce `g.items` with `length` or a
projection such as `sum([r.demand for r in g.items])`.

The callable form is `ValueExpressionKind::GroupBy { rows, field }`, where
`field` is a `Located<String>` and `rows` is the sole expression operand. The
five value APIs and native-value APIs use the shared analysis and evaluator.
Type discovery retains possible original row schemas in `items` without
claiming a group count or assigning rows to particular keys. A schema-bearing
empty projection retains these facts; an empty literal invents no key or row
schema. Missing field facts remain deferred until execution, so a skipped
missing field does not fail. Known incompatible keys still fail preflight.
Missing fields and key-type refusals point to the selector; non-array input and
non-record rows point to the grouping call. Child failures retain their ranges.

Grouping evaluates its rows once. Comparisons, new containers and slots, copied
group keys and labels use the active budget. Rows move into the new groups;
their values are not copied again. Subsequent snapshots and rendering use the
same allowance. See [diagnostics and limits](diagnostics-and-limits.md).

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
