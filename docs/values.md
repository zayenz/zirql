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

## Exact string matching

`contains(text, fragment)`, `starts_with(text, prefix)` and
`ends_with(text, suffix)` return fixed Booleans. Both arguments must be strings;
there is no coercion, case folding, normalization or regex interpretation.
Empty search strings match every string, including an empty text.

```text
contains("Ångström", "ström")  → true
starts_with("Ångström", "ång") → false
ends_with("Ångström", "")      → true
contains("é", "é")            → false
```

Both operands evaluate in written order. Known non-string operands fail
preflight, including in skipped branches. Matching borrows existing string
storage. The active request budget admits a byte-scan bound before matching:
text plus search byte lengths for nonempty `contains`, or the smaller byte
length for prefix/suffix checks. An empty search needs no scan work. Type-error
diagnostics also spend this budget. Ordinary operand evaluation and result
admission retain their existing charges.

Rust callers use `ValueExpressionKind::Binary` with
`ValueBinaryOperator::{Contains, StartsWith, EndsWith}`. The common reader lowers
the three calls to those same operators. Domain name extraction remains with
the host. These helpers are available in the development Git revision;
published alpha.5 predates them.

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
User function signatures use the [shared function reader](embedding-programs.md#call-typed-pure-functions). Optional external parameter transport remains deferred.

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

## Sorting and optional extrema

`sort(xs)` sorts a positional array of scalars. `sort_by(rows, "field")` sorts
records by a static field label. Both are ascending and stable. The optional
literal option `descending=true` reverses key order and keeps input order for
equal keys; `descending=false` is the default. Other record fields and stored
field order are retained, and rows may have different schemas.

`min(xs)` and `max(xs)` return one original scalar. `min_by(rows, "field")` and
`max_by(rows, "field")` return one original record. Empty input returns absence;
equal keys select the first input value. Empty sorting returns an empty array.
Use an explicit default when absence needs a replacement:

```text
sort([2, 1, 2, 1])                  → [1, 1, 2, 2]
sort([2, 1], descending=true)       → [2, 1]
min([]) default 0                  → 0
max([2, 1, 2, 1])                  → 2
```

Keys must be present and homogeneous: integers, finite floats, strings, or
members of one nominal type. Integers and floats do not mix. Strings use exact
lexicographic order without normalization. Nominal members use their declared
order and retain their identity. Booleans and native handles need an explicit
supported projection before ordering. Keyed arrays need explicit conversion,
for example `[cell.value for cell in entries(a)]`, to choose positional values.

A missing field, absent key or incompatible key fails at the selector range;
scalar key failures use the call range. Later bad keys still fail an extremum
request even after an earlier winner. Known incompatible kinds fail preflight
in skipped branches; missing data and presence are checked when executed. To
choose a default key, first project records with a field such as
`lookup(row, "score") default 0`, then order that field.

The Rust forms are `ValueExpressionKind::Sort { array, field, descending }` and
`Extremum { operator, array, field }`, with `ValueExtremum::Min` or `Max` and an
optional located static field. They use the same value, native-value and
prepared-function APIs. Result facts describe possible reordered elements,
not the input positions; selected values keep their declarations. These are
constructed-value calls. Host-native stages such as bare `sort` or a host's
single-selector `sort_by(name)` keep their domain behavior.

Sorting extracts each key once and uses a fallible stable merge with O(n log n)
key comparisons and O(n) temporary descriptors/positions. Comparisons and
scratch admission share the request budget. Payloads move by the resulting
permutation; shared input needs one owned output copy. Extrema instead scan
once and retain only the selected row/key, borrowing shared input and copying
only the selected result when an owned boundary requires it. See
[resource charges](diagnostics-and-limits.md#what-uses-the-allowance).

These helpers are available in the development Git revision; published
alpha.5 predates them. Host data-edit whitelists and serialization policies
remain separate from report construction.

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


## Callable patterns and lexical blocks

The Rust value APIs support lexical blocks and first-success value matching.
A `Block` evaluates immutable pattern bindings in order, then returns its final
expression. Patterns support nested names, wildcards, exact tuples, records,
present scalar literals and optional presence/absence. Tuple patterns accept
tuples of the exact length. They do not accept positional arrays. Record labels
must actually be stored: an omitted optional field is a shape mismatch. Without
`ignore_rest`, the stored label set must match exactly; ignored fields are never
copied into a rest value.

A binding mismatch fails at its pattern without installing any names. In a
`ValueMatch`, the same mismatch tries the next arm. Literal kinds must agree;
an integer literal pattern does not match a float. A `Present` pattern views an
existing present value with its underlying contract. It introduces neither a
wrapper nor another absence state. `Absent` matches the existing absent value.

Match arms run in written order, require a final unguarded wildcard, and can
have known-Boolean guards. False continues; Unknown, non-Boolean and evaluation
errors fail. Only the selected result runs. Arm names remain local to that arm,
and every written guard and result receives preflight checks. Native capture
matching still enumerates correlated records through its existing API.

Constructed selections share their retained owner and child path. Native record
and tuple patterns require explicit host views; native handles keep their
identity and source coordinates. Typed parameters admit each whole argument
before matching the complete parameter set. See the
[embedding guide](embedding-programs.md#destructure-values-and-select-match-arms)
for the callable types and host hooks. The common value reader accepts
`match (subject) { pattern [if guard] => value, ..., _ => fallback }` and
conditional blocks with immutable local lets. `read_pattern_function_declaration`
adds destructured typed parameters; the existing name-only reader is unchanged.
Consumer guides describe which frontend entry points expose these forms.

Patterns spell renaming as `{field: local}`, discards as `_`, and ignored extra
fields as a final `..`. Quoted static field labels are supported. Tuple values
use `(first, second)` and remain distinct from arrays. In a function body or
conditional branch, braces hold local lets and a final expression. A match arm
can return an ordinary JSON record directly or use a block such as
`{ let answer = cost + 1; answer }`. Native `match { from ... }` still enumerates
captures; value matching selects one result.


## Callable loop values

`Loop` returns the value selected by its innermost `Break`. Its fresh state names
are immutable during each body execution. `Continue` computes a complete next
state left-to-right against the old slots and checks every established contract
before replacing any slot. Blocks and value-match bindings remain local to one
iteration. Nested loops own their own transfers; called functions cannot transfer
to a caller's loop.

Preparation rejects outside-loop transfers, wrong state arity and any body path
that falls through. Transfers appear only in terminal block, conditional or match
positions. Fixed runtime contracts preserve optional/schema/nominal information;
collection lengths can vary. Unannotated absence and an untyped empty collection
need an explicit initializer contract. Selected break values still receive the
enclosing function's result check.

These are callable representations. The [embedding guide](embedding-programs.md#repeat-with-explicit-state)
contains examples and explains the later textual integration boundary.


## Native computational collections

The callable Rust function API adds `DeclaredType::Queue(Box<DeclaredType<K>>)`
and `DeclaredType::IdentitySet(K)`. A queue carries its whole element contract;
an identity set accepts one native kind. Existing sorted scalar and nominal
sets retain their ordering and operations.

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
library values. Their budgeted operations preserve retained aliases, and returned
wrappers remain usable after the function frame ends. Neither collection is a
data value: direct scalar projection, rendering and serialization must fail
with a located error. Identity-set iteration and general set algebra are absent.
These operations currently have callable representations only; no common
worklist expression grammar or consumer frontend is supplied by this increment.
