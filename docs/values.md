# Values and comprehensions

A value expression computes a scalar or builds a collection. The word model
supplies `words = ["ash", "apple", "birch", "apple"]`; the examples on this page
use that binding or literal inputs. The final sections introduce values and
operations that need additional host support. For the Rust representations, see
[value evaluation](value-runtime.md).

<span id="expression-trees"></span>

## Literals and access

| Value | Example | Notes |
| --- | --- | --- |
| Boolean | `true`, `false` | Conditions require a known Boolean |
| Integer | `42`, `-3`, `0x2a`, `0o52` | Signed 64-bit, with checked arithmetic |
| Float | `1.5`, `2.0e3` | Finite binary64 values |
| String | `"apple"` | Double-quoted text |
| Absence | `null`, `<>` | One absent value with two accepted spellings |
| Array | `["ash", "apple"]` | Ordered; duplicates and different element types are allowed |
| Tuple | `("ash", 3)` | Fixed positions; distinct from an array |
| Dictionary | `{"word": "ash", "count": 1}`, `{{"ash", "apple"}: 2}` | Constructed keys and values; insertion order |
| Set | `{3, 1, 3}`, `{[1], (1,), {1}}` | Constructed elements; first occurrence wins |

Use a field label or a one-based position to select a value:

```zirql
{"word": "ash", "count": 1}.word
```

The result is `"ash"`. `words[1]` is `"ash"`, and `(2, 5).2` is `5`.
Tuple positions use a dot; array positions and computed dictionary keys use brackets.
`{"word": "ash"}["word"]` is `"ash"`. Accesses compose from
left to right: `({"rows": [(2, 5)]}, 0).1.rows[1].2` is `5`. Position zero,
a position beyond the collection and a missing field are errors. Use `lookup`
when a missing entry is expected.

Strings are scalar values. Array operations such as `length` do not implicitly
operate on their characters. A host can expose a byte count or another text
measure when its model needs one.

## Reuse a record with changed fields

`record with {field: expression, ...}` returns a record with replacements for
existing string fields. Labels are static names or double-quoted strings.
Replacement order does not move fields in the result. The original record and
any aliases keep their values, including native children and source coordinates.

The base evaluates once, followed by each replacement in written order. All
replacements use the surrounding scope. A label in the update list introduces no
binding, and later replacements cannot read an earlier replacement by its label.
Use the original record's name to read its old fields. Repeated or missing target
labels fail. The result must satisfy any retained explicit record schema,
including its field contracts. Ordinary structural fields can change type.
An empty update returns the same layout.

### Compare capacity scenarios

Run this complete report with the local CSV host:

```sh
cargo run --locked --example csv -- '
let original = {"name": "cpu", "limit": 8, "used": 6};
let expanded = original with {limit: original.limit + 4};
{"original": original, "expanded": expanded,
 "remaining": [c.limit - c.used for c in [original, expanded]]}
' examples/csv/people.csv
```

```json
{"original":{"name":"cpu","limit":8,"used":6},"expanded":{"name":"cpu","limit":12,"used":6},"remaining":[2,6]}
```

This report compares two ordinary values. It does not change source data.
Updates support the existing string-field record representation only. They cannot
insert or remove fields, update a general dictionary or replace keyed-array
cells. Keyed capacity updates remain a separate design question, including how
keys, missing cells and declared axes should behave.

The shared reader, callable evaluator and local word/CSV hosts support `with`
in this development checkout. Installed Zincite and Zirium frontends need
[consumer adoption](host-integration.md#immutable-record-adoption) before they can
advertise this syntax. This ordinary value operation adds no symbolic CP form.

## Operators and precedence

The common reader applies these precedence levels, strongest first:

| Level | Operations |
| --- | --- |
| Access and record updates | `a[i]`, `a[i, j]`, `tuple.1`, `row.field`, `row with {field: value}` |
| Optional fallback | `default` |
| Prefix | `not`, unary `+` and `-` |
| Multiplication and division | `*`, `/`, `div`, `mod` |
| Addition and subtraction | `+`, `-` |
| Inclusive integer range | `..` |
| String concatenation | `++` |
| Ordering | `<`, `<=`, `>`, `>=` |
| Equality | `==` (also `=`), `!=` |
| Boolean conjunction | `and`, `/\` |
| Boolean disjunction | `or`, `\/` |

Binary operators at the same level associate to the left. Use parentheses to
make a fallback boundary clear: `(lookup(row, "count") default 0) + 1`.
Parentheses also group expressions; a comma makes a tuple.

Whitespace, `#`/`%` line comments and nonnested `/* ... */` comments separate
tokens. Strings use double quotes and JSON escapes, including Unicode escapes;
literal newlines, carriage returns and tabs are retained. Single quotes surround
nonempty identifiers: `'row count'` names one binding, and `'if'` remains a name.
Names compare without their quotes; source ranges include them. Integer literals
also accept `0x`/`0X` hexadecimal and `0o` octal digits, including the signed minimum
`-0x8000000000000000`. Hexadecimal floats and integer overflow are refused.

## Checked arithmetic and conditions

Overflow, division by zero and nonfinite float results are errors. `/` produces
a float, including for two integer operands. Integer `div` truncates toward
zero; `mod` has the dividend's sign. The minimum signed integer divided by `-1`
also fails. Mixed integer/float arithmetic requires the integer to have an exact
binary64 representation. Strings receive no numeric coercion. A nonzero floating
result that underflows to zero fails.

`and` and `or` short-circuit. A conditional evaluates its condition, then only
the selected branch:

```zirql
if length(words) > 0 { words[1] } else { "no words" }
```

The same expression can use `if length(words) > 0 then words[1] else "no words" endif`.
Additional branches use `elseif condition then value`; both spellings require a
final `else`. This returns `"ash"`. With empty input it returns `"no words"` without trying
to access position 1. Every branch still receives scope and known-type checks.
See [functions and control flow](functions-and-control-flow.md) for local
bindings, pattern matching and recursion.

## Exact string matching

`contains(text, fragment)`, `starts_with(text, prefix)` and
`ends_with(text, suffix)` take two strings and return a Boolean. They perform
no case folding, normalization or regular-expression matching.

| Expression | Result |
| --- | --- |
| `contains("apple", "pp")` | `true` |
| `starts_with("Apple", "a")` | `false` |
| `ends_with("ash", "sh")` | `true` |
| `contains("ash", "")` | `true` |

An empty search string matches any string, including empty text. Canonically
equivalent Unicode spellings can still differ: a precomposed accented character
and a letter followed by a combining accent are different strings.

## Absence, lookup and defaults

Absence is data. It is distinct from a failed operation and from an unknown
native predicate. False, zero, an empty string and an empty collection are all
present values.

`lookup(container, key)` returns absence for a valid missing entry. Dictionary keys
can be any constructed value, positional keys are one-based integers, and keyed
arrays retain their declared key types and rank. Incompatible containers or keys
outside a declared contract fail. A record schema still restricts its string labels.

| Expression | Result |
| --- | --- |
| `lookup({}, "word")` | `null` |
| `lookup({"word": null}, "word")` | `null` |
| `has_key({}, "word")` | `false` |
| `has_key({"word": null}, "word")` | `true` |
| `occurs(false)` | `true` |
| `absent(null)` | `true` |

`has_key` distinguishes a missing entry from a stored absence. `occurs` tests
whether a value is present, and `absent` returns the opposite answer.

```zirql
lookup({"word": "ash"}, "label") default "unlabelled"
```

The result is `"unlabelled"`. A default evaluates its left operand once, then
only evaluates the fallback if the value is absent. It does not catch errors.
For example, `(1 div 0) default 0` fails at the division.

Arithmetic, ordering and conditions need present values. Two absences compare
equal, and an absence is unequal to a present value, subject to compatible
underlying types. Declared optional integers and optional strings do not become
compatible merely because both happen to be absent.

## Positional comprehensions

A comprehension evaluates a source, introduces a local name and constructs a
result for each selected element:

```zirql
[{"word": w, "label": "word:" ++ w}
 for w in words if starts_with(w, "a")]
```

It returns records for `ash`, `apple`, `apple`, in that order. The `++` operator
concatenates strings. Duplicate input values produce duplicate records.

The MiniZinc spelling has the same meaning:

```zirql
[(word: w, label: "word:" ++ w)
 | w in words where starts_with(w, "a")]
```

The head can use all binders. Sources and guards are read in written order.
Grouped `x, y in source` binders traverse the same source as a Cartesian product.
In the MiniZinc form, commas separate generators and `name = value` binds one
value once per preceding combination:

```zirql
[(x, y, total) | x, y in [1, 2], total = x + y where total > 2]
```

This returns `[(1, 2, 3), (2, 1, 3), (2, 2, 4)]`. The common form uses repeated
`for ... in ...` clauses; it has no `for pattern = value` spelling. Both forms
accept shared destructuring patterns. A generator pipe belongs to its collection;
group native pipelines inside a head or source, and place collection suffixes
after its closing bracket.

Later generators can use earlier names. Each source is evaluated before its
own pattern names become available; its filter can use those names. For example,
`[a + b for [a, b, ..] in [[1, 2, 3], [4, 5]]]` returns `[3, 9]`. A
pattern mismatch in an ordinary generator is an error even when a later filter
would be false. Use `case` below when mismatches should skip candidates. The rightmost
generator varies fastest:

```zirql
[{"word": w, "copy": n} for w in ["ash", "apple"] for n in 1..2]
```

This produces `ash/1`, `ash/2`, `apple/1`, `apple/2`. The range includes both
endpoints. Local names cannot shadow an outer binding. `_` discards an element
without introducing a name.

Arrays assign fresh one-based positions. Filtering positions 1, 2 and 4 does
not retain a gap at 3. All candidates cost work, including rejected ones, and
the result is fully constructed before an enclosing reducer runs.

## Select matching values

Use `for case pattern in source` to select values by their structure, literal
fields or optional presence. This CSV example selects present measurements from
mixed records. Run it from this Zirql checkout:

```sh
cargo run --locked --example csv -- '
let records = [
  {"kind": "measurement", "name": "ash", "reading": 12},
  {"kind": "note", "name": "memo", "reading": 99},
  {"kind": "measurement", "name": "birch", "reading": null},
  5,
  {"kind": "measurement", "name": "oak", "reading": 7}
];
[{"name": name, "reading": reading}
 for case {kind: "measurement", name, reading: present(reading), ..} in records]
' examples/csv/people.csv
```

The JSON result is:

```json
[{"name":"ash","reading":12},{"name":"oak","reading":7}]
```

The note fails the literal pattern, birch has an absent reading and the scalar
has the wrong shape. The MiniZinc spelling selects the same values from
`records`:

```zirql
[{"name": name, "reading": reading}
 | case {kind: "measurement", name, reading: present(reading), ..} in records]
```

A mismatch skips that candidate's filter, later sources and head. Successful
bindings stay local to the candidate and follow the ordinary evaluation order.
For grouped binders, `case` applies to the whole clause: a mismatch in any binder
skips that combination. The single-value `pattern = value` generator, ordinary
`in` generators and `let` bindings keep strict pattern errors.

Only pattern mismatches are skipped. Host inspection, admission and runtime
errors still fail, as do Unknown guards. Every written source, filter and head
receives the usual scope, type and purity checks, including when no candidate
matches. Rejected candidates spend work; grouped expansion is charged before
matching. Work and result limits fail the request without a partial report.

These forms run through the CSV and word examples in this checkout and through
`ValueGenerator::In { case: true, .. }`. Consumer dependency and builder changes
are described under [selective generator adoption](host-integration.md#selective-generator-adoption).

## Array summaries and requirements

| Call | Meaning | Empty input |
| --- | --- | --- |
| `length(a)` | Collection cardinality, including present keyed cells | `0` |
| `sum(a)` | Checked integer sum in input order | `0` |
| `forall(a)` | Every entry is true | `true` |
| `exists(a)` | At least one entry is true | `false` |

`length` accepts positional arrays, sets and dictionaries, plus present keyed
cells. The other reducers keep their positional/keyed array contracts: `sum`
requires integers, and `forall` and `exists` require known Booleans. Keyed
summaries do not fill missing cells. Strings and tuples are refused.

```zirql
length([w for w in words if contains(w, "p")])
```

This returns `2`. The comprehension constructs both matches before `length`
counts them. Boolean reducers likewise receive a completed array and inspect
all its elements; an earlier decisive Boolean does not skip later construction
or errors.

`require(condition, value, "message")` returns `value` when the condition is
true. Otherwise it fails with the literal message and does not evaluate `value`:

```zirql
require(length(words) > 0, words[1], "expected at least one word")
```

The result is `"ash"`. On empty input the requirement fails before the access.
Scope and known-type checks still cover the unselected value.

## Complete windows and consecutive runs

`windows(items, size)` returns an array of complete overlapping windows with
stride one. `items | windows(size)` is equivalent. The source must be a
positional array and `size` must evaluate to a positive integer. Windows are
eager arrays in input order. A source shorter than `size`, including empty
input, returns `[]`; there is no padding or configurable stride.

```zirql
windows([1, 2, 3, 4], 2)
```

This returns `[[1, 2], [2, 3], [3, 4]]`. Repeated values remain repeated.
Expansion and copies spend the request's work, result and nesting allowances;
exhaustion fails the request without returning a truncated result.

`runs_by(rows, "field")` and `rows | runs_by("field")` group consecutive rows
with structurally equal field values. Each run is a record with `key` and
`items`. Runs and their members retain input order and duplicates. The selector
is a literal field name, with the usual missing-field error. Keys may be any
constructed values, including sets and dictionaries; their structural equality
ignores set/dictionary insertion order. A later equal key starts another run
when a different key intervenes. Empty input returns `[]`.

Both operations accept positional sources only, with no selector callbacks.
They are available through the shared reader and callable evaluator in this
development checkout, including the local word and CSV examples. See the
[runnable word and operation-name reports](examples.md#find-adjacent-word-pairs)
and [consumer adoption requirements](host-integration.md#sequence-neighbourhood-adoption).

## Align two record sequences

`align(left, right, by="field")` compares two positional sequences of records.
Each row must have a present value in the selected field, and each side must
have unique keys. The result is an array of records with `key`, `before` and
`after`. An unmatched side is absent. Guard it with `occurs` or `absent` before
selecting its fields.

```zirql
align([{"id": "a", "ms": 12}, {"id": "b", "ms": 7}],
      [{"id": "c", "ms": 9}, {"id": "a", "ms": 10}], by="id")
```

This produces:

```json
[{"key":"a","before":{"id":"a","ms":12},"after":{"id":"a","ms":10}},{"key":"b","before":{"id":"b","ms":7},"after":null},{"key":"c","before":null,"after":{"id":"c","ms":9}}]
```

Rows follow left input order, then right-only keys in right input order. A
matched row keeps the left key's representation and both original records.
Keys use structural identity: set membership and dictionary associations ignore
insertion order, while arrays and tuples retain positions. Different value
kinds stay distinct, including integer and float keys. Keys may contain nested
absence; the selected key itself must be present. Hosts can retain native keys
through their existing identity and hashing rules.

Both operands receive normal preparation, including when one is empty. They
evaluate eagerly once, left then right. Empty sides contribute unmatched rows;
both empty returns `[]`. Missing fields, absent keys, duplicate keys and
incompatible source or row kinds fail with located errors. The selector is a
literal field name; this operation has no selector callback or pipeline suffix.
Work, indexing, comparisons and result construction share the request budget;
exhaustion fails the request.

The shared reader, callable evaluator and local CSV/word hosts support this call
in the development checkout. Run the [benchmark comparison](examples.md#compare-benchmark-tables)
for a report that reads timings only from present sides. Consumer adoption is
[separate](host-integration.md#record-alignment-adoption); `align` is an ordinary
value operation and adds no symbolic CP support.

## Stable record groups

`group_by(rows, "field")` groups an array of records by a literal field label.
Each result has a `key` and an `items` array. Groups follow the first occurrence
of each key, and members keep their order and duplicates.

```zirql
group_by([{"word": "ash"}, {"word": "apple"}, {"word": "ash"}], "word")
```

The first group has key `"ash"` and two records; the second has key `"apple"`
and one record. Empty input produces an empty array.

Keys must all be integers, strings or members of one nominal type. Missing
fields and incompatible keys fail. A record can retain other fields, and those
fields can differ between members. See [word occurrences](examples.md#keep-the-occurrences-in-each-group)
for a report that retains the positions of repeated words.

## Sorting and optional extrema

`sort(a)` orders an array by its scalar values, a set by structural value order
and a dictionary by structural key order. Nested collection keys are supported.
Sorting retains the collection kind. `sort_by(rows, "field")` orders
array rows or dictionary values by a literal field label; `sort_by(d, value)`
orders dictionary values directly. All sorts are ascending and stable.
`descending=true` reverses key order while preserving input order within ties.

```zirql
sort(words)
```

The result is `["apple", "apple", "ash", "birch"]`. Strings use exact
lexicographic order without normalization or locale-specific collation.

`min` and `max` select an original scalar; `min_by` and `max_by` select an
original record. Ties select the first input value. Empty input yields absence:

```zirql
min([]) default "no minimum"
```

This returns `"no minimum"`. Sorting an empty array returns an empty array.

Array ordering and `sort_by`/extremum selectors require present, homogeneous
integers, finite floats, strings or members of one nominal type. Integers and
floats do not mix in those scalar selector contracts. Boolean or native values
need a supported projection there. A missing field, absent key or later
incompatible selector key fails the whole operation, including an extremum that
already has a candidate winner. Project an explicit default if missing data
should have an order. Keyed arrays need a conversion such as
`[cell.value for cell in entries(a)]` before these operations. General set and
dictionary-key sorting instead uses structural order, including Boolean, absent
and nested collection values.

## Constructed collection pipelines

A positional array can select elements with a Boolean expression:

```zirql
people | filter(p => p.active == "yes")
people | filter(_.active == "yes")
[p for p in people if p.active == "yes"]
```

These retain the same rows in input order, including duplicates. The optional
name and `_` denote the same element. Comparisons, Boolean composition, lexical
captures and permitted pure calls use the ordinary expression rules. A predicate
must produce a known Boolean. Known incompatible types, undefined names and
forbidden operations fail before iteration, including for typed empty input.
A keyed array needs an explicit conversion before filtering; filtering never
silently renumbers keyed cells.

Nested filters bind `_` anew. Give the outer element a name when it is needed
inside another predicate:

```zirql
people | filter(p => length(["compiler"] | filter(_ == p.team)) > 0)
```

Ordinary comprehension filters, conditionals, requirements and value-match
guards see the enclosing predicate scope. `_` is unavailable outside that
scope and still means discard in patterns and generator binders.

Bare `filter` and `count` read the host's immutable constructed input:

```zirql
{"beginning_with_a": filter(starts_with(_, "a")) | count, "total": count}
```

The word host returns `{"beginning_with_a": 3, "total": 4}`. An explicit source,
such as `["apple", "birch"] | filter(w => starts_with(w, "a")) | count`, returns
`1`. Empty and no-match arrays count as zero. The compact
`filter(starts_with("a"))` spelling uses the same expression machinery.

The `sort` suffix applies the same ascending sort to arrays, sets and
dictionaries, retaining their kind. Positional arrays also support stable
`unique` and terminal `tally`:

```zirql
[w for w in words if starts_with(w, "a")] | sort | unique
```

This returns `["apple", "ash"]`. Without sorting, `| unique` retains
`["ash", "apple"]`. Unique accepts homogeneous Booleans, integers, finite
floats, strings or members of one nominal type. Floating `-0.0` and `0.0`
compare equal. Native handles, absence, records, tuples, nested collections,
keyed arrays and sets are unsupported elements. Values are never converted to
strings implicitly.

```zirql
[w for w in words if starts_with(w, "a")] | tally
```

Tally accepts constructed values and returns a dictionary of checked signed
integer counts. String input gives `{"ash": 1, "apple": 2}` with fields in
first-seen order. No further stage may follow tally. Empty untyped arrays yield
`[]` for sort/unique and `{}` for tally. Unique retains its scalar-only rules.

Collections can be tally keys:

```zirql
[{words[i], words[i + 1]} | i in index_set(words) where i < length(words)] | tally
```

For `ash apple birch apple`, the language value is:

```zirql
{{"ash", "apple"}: 1, {"apple", "birch"}: 2}
```

The reversed pair `{"birch", "apple"}` joins the second group. Groups follow
first occurrence, and each key keeps its first iteration order. Equal adjacent
words produce singleton keys. Tally also accepts tuples, lists, dictionaries,
mixed scalar kinds and absence. It uses the structural key equality described
below, without converting keys to strings. Empty input known to contain sets
returns an empty general dictionary; ambiguous `[] | tally` and empty string
input retain the empty string dictionary `{}`.

A pipe applies to the completed expression, after arithmetic, Boolean and
`default` operators. A completed comprehension or conditional can pipe directly.
A host that knows a binding is constructed accepts its bare name:

```zirql
words | sort | unique
(make_words()) | tally
```

The word and CSV examples supply this binding fact. The second form requires
a host that registers `make_words`; grouping also works for bindings. Hosts
resolve native sources and function results using their own type facts. Group
any pipeline result before using it in a larger expression, for example
`length(([w for w in words] | unique))`. Count produces a scalar; a subsequent
collection stage fails at that stage. The richer `sort(a, descending=true)` and
`sort_by` call forms keep their existing contracts.

## Construct a capacity report

A capacity has a name, a limit and a sequence of demands. `flat_map` visits the
capacities in order; its inner `map` constructs one record per demand. The outer
name `c` stays available while the inner `_` denotes the current demand.

Run this command from a checkout containing the mapping stages:

```sh
cargo run --locked --example words -- '
[
  {"name": "cpu", "limit": 8, "demands": [2, 3]},
  {"name": "gpu", "limit": 6, "demands": [4]}
]
| flat_map(c => _.demands | map({
    "capacity": c.name,
    "demand": _,
    "remaining": c.limit - _
  }))
'
```

The word example prints Rust value notation:

```text
Array([Record([("capacity", String("cpu")), ("demand", Integer(2)), ("remaining", Integer(6))]), Record([("capacity", String("cpu")), ("demand", Integer(3)), ("remaining", Integer(5))]), Record([("capacity", String("gpu")), ("demand", Integer(4)), ("remaining", Integer(2))])])
```

`map` retains one result for each input item. `flat_map` concatenates one level
of positional results, including empty results, without changing their order.
For example, `[1, 2] | map([_])` returns `[[1], [2]]`, whereas
`[1, 2] | flat_map([_])` returns `[1, 2]`. A scalar or set body result is an error
for `flat_map`; use an explicit positional construction when needed. Sets,
dictionaries, keyed arrays and tuples are also refused as stage inputs.

Bodies use the same pure expressions and strict patterns as scoped filters,
without a Boolean requirement. Empty inputs still receive full body preflight.
Nested arguments bind `_` anew, and named captures remain lexical. The eager
result is buffered under the shared work, result, allocation and stream limits.
Native values retain their owners when the host supplies positional views and
native report construction. The [embedding guide](embedding-programs.md#scoped-mapping-in-rust)
explains those hooks and the separate consumer integration requirements.

## Nominal members and sets

A nominal type gives its members an identity as well as a name. A host might use
one for a fixed set of word categories. Identity includes both the type and
member, so identical printed names from different types are not equal. Members
use declaration order. Arithmetic on members and mixed nominal types fail.

Sets accept constructed values, including tuples, lists, sets and dictionaries.
They remove structurally equal elements while retaining each first occurrence.
For example, `{{1, 2}, {2, 1}, [1, 2], (1, 2)}` contains three elements: the
nested sets are equal, while the list and tuple are distinct kinds. Sets compare
membership independently of stored order. Mixed scalar kinds remain distinct,
so `{1, 1.0, true}` contains three elements.

Bare braces construct sets: `{3, 1, 3}` iterates as `3, 1`, and
`{n mod 3 for n in 1..5}` iterates as `1, 2, 0`. Empty braces mean a dictionary;
use `set<value>()` for an empty general set. `set<int>()`, `set<bool>()`,
`set<float>()` and `set<string>()` retain strict scalar element contracts.
A host can supply a typed empty nominal set through the Rust API; the common
reader does not resolve nominal type names.

Inferred homogeneous scalar sets keep their precise `SetElementType`; nested
or mixed sets use `SetElementType::Value`. String equality uses exact case and
bytes. `sort` uses structural order for mixed and nested elements, keeping
scalar kinds distinct. Lists and tuples compare lexicographically; sets compare
canonically ordered members and dictionaries compare canonically ordered
key/value associations. Rust hosts construct set storage with
`elements: vec![...].into()`. The `SetElements` wrapper caches a structural
membership index and invalidates it when the stored elements change.
Serialization remains an explicit host boundary. Hosts choose an encoding that
preserves the required information or refuse unsupported values.

## Keyed arrays

An indexed array addresses its cells with integer or nominal keys. For example,
row keys `2` and `5` do not imply that rows `3` and `4` exist:

```zirql
[2: 20, 5: 50]
```

The keys establish one dimension, ordered numerically. Nominal keys use the
host's member identity and declaration order. To retain a particular order or
keys without a cell, declare the full row keys after the bracket:

```zirql
[2: 20, 5: 50] axes([5, 2, 9])
```

The result retains keys `5, 2, 9` and stores cells `5: 50`, then `2: 20`.
Key `9` has no cell. Each ordered set of keys is called an axis; `axes(...)`
supplies one source per dimension. Sources are integer/member arrays or typed
sets. They run once, left to right, in surrounding scope before any cells or
generators. `axes` is contextual construction syntax, not a built-in function.

A written integer key can start a positional tail: `[3: 7, 8]` supplies cells
`3: 7` and `4: 8`. Every omitted key needs a checked successor of the preceding
written integer key. Nominal or computed keys require explicit following keys.

Matrices use `[| 1, 2 | 3, 4 |]`, with one-based row and column keys. Explicit
columns end each key with a colon, followed by a pipe; a row key precedes its
cells. For example, `[| 2: 1: | 4: 42, 41 | 3: 32, 31 |][3, 1]` returns `31`.
Rows must be rectangular, and empty matrices without established axes fail.
`a[i, j]` and `a[(i, j)]` use the same tuple index.

A tuple supplies row and column keys for a two-dimensional value:

```zirql
[(i, j): i + j for i in [5, 2] for j in [8, 1] if j == 8]
    axes([5, 2, 9], [8, 1, 4])
```

Only cells `(5, 8)` and `(2, 8)` exist. Zirql permits these sparse values.
Cells are stored in row-major order, with the rightmost dimension varying
fastest. A duplicate full key, duplicate axis key, wrong rank, incompatible
nominal type or key outside its declared domain fails. String keys belong to
[dictionaries](#literals-and-access).

Empty results retain declared dimensions and key types:

```zirql
[] axes(set<int>(), [8, 1])
[i: i for i in [2, 5] if false] axes([5, 2, 9])
```

The first has two dimensions and no rows. The second has one dimension with
three possible keys and no present cells. `[]` by itself remains positional.
An untyped empty source such as `[]` cannot establish a dimension's key type.
A host can supply a typed empty nominal set without losing its identity.

A host may require contiguous domains or complete rectangular coverage, as
Zincite does for constructed data. Those checks do not fill or compact cells.
See [value evaluation](value-runtime.md#keyed-construction) for the Rust fields
and the separate lazy empty fallback. Without explicit axes, a direct key binder
can recover an independent generator domain when no cells are emitted:
`[i: i | i in set<int>()]` retains an empty integer axis. A tuple of direct
key binders can recover each independent domain. Dependent or unestablished
domains do not invent a shape. Explicit `axes(...)` always takes precedence.

`index_set(a)` returns a typed set of one-dimensional array keys. For a
positional array this is `1..length(a)`, including a typed empty integer set
for `[]`. For a rank-one keyed array it uses the declared axis, including keys
with no stored cell, and retains integer or nominal identity for empty axes.
The result retains the declared axis order, even when it differs from numeric
or nominal declaration order. Inputs of another kind or rank fail at the call.

```zirql
[words[i] for i in index_set(words)]
```

This returns the words in positional order. `index_set([2: 20, 5: 50]
axes([5, 2, 9]))` returns the ordered integer set `{5, 2, 9}`.

## Keyed entries and exact pairing

`entries(a)` exposes present cells as a positional array of `key`/`value`
records in stored order. Rank-one keys are integer or nominal scalars;
higher-rank keys are tuples. Empty input gives an empty array.

`zip_exact(a, b)` requires identical ordered axes and present full keys, including
sparse coverage. It retains those axes and keys, replacing each cell with a
tuple of its left and right values. Equal cell counts alone do not establish
alignment. Equal empty inputs retain their typed axes.

Suppose the host supplies two rank-one keyed arrays, both with keys `2` and `5`.
The binding `before` has values `3` and `7`; `after` has values `4` and `9`.
Pair their cells with:

```zirql
entries(zip_exact(before, after))
```

The result is equivalent to `[{"key": 2, "value": (3, 4)}, {"key": 5, "value":
(7, 9)}]`. These bindings must be supplied by the host; they are not part of the
word model. Neither helper fills, truncates, reindexes or writes data.

## Native operands and extraction

A native query can appear inside a value expression when the host supplies a
projection. For example, suppose `word` holds one native word occurrence and the
host supplies a `text` stage that returns its spelling:

```zirql
one(word | text)
```

`one` requires a positional array containing exactly one element. Zero or
multiple elements fail. It does not mean "take the first". Projected strings
remain an array until extracted; a native count is converted to a signed integer
only if it fits.

<span id="immutable-reads-and-owned-results"></span>
<span id="callable-patterns-and-lexical-blocks"></span>
<span id="callable-loop-values"></span>
<span id="native-computational-collections"></span>

Functions, patterns and the availability of other control-flow forms are covered
in [functions and control flow](functions-and-control-flow.md). Rust ownership,
prepared reads and native worklist values are described in
[value evaluation](value-runtime.md).

<span id="selected-collection-syntax-and-ordering"></span>

## Dictionaries and collection order

An outer colon distinguishes a dictionary from a set. Each key is an expression:
`{"z": 2, "a": 2, "m": 1}` uses quoted strings, while `{w: 1 for w in words}`
reads each `w` as its key. Static quoted fields retain their structural schemas.
`.field` selects a static string label; bracket access and `lookup` accept any
constructed key. Empty `{}` is an empty dictionary.

Dictionaries preserve entry insertion order. Each entry evaluates its key,
checks for a duplicate, then evaluates its value. A duplicate fails before that
value runs. Keys and values can be scalars, absence, tuples, lists, sets or
other dictionaries. For example:

```zirql
{[1, 2]: "list", (1, 2): "tuple", {1, 2}: "set", {"x": 1}: "dictionary"}
lookup({{1, 2}: "pair"}, {2, 1})
```

The lookup returns `"pair"`. Lists and tuples compare by position; sets compare
by membership; dictionaries compare key/value associations. Set and dictionary
iteration order does not affect key identity. Different value kinds remain
distinct, including integer `1` and float `1.0`. Floating `-0.0` and `0.0`
have the same identity and hash. No key is converted to text.

Rust adapters receive all-string dictionaries as `ConstructedValue::Record`
and other dictionaries as `ConstructedValue::Dictionary(Dictionary)`. The
`Dictionary` storage exposes ordered entry pairs and caches a structural hash
index for repeated lookup; `entries.into()` constructs it from an owned vector. Empty `{}` remains a record. `dict<K, V>` declares
key and value contracts; `value` accepts any constructed value. Record schemas
remain available for fixed string fields. See [value evaluation](value-runtime.md).

Repeated `for` clauses and filters use the same scope and traversal rules as
array comprehensions:

```zirql
{k: {n mod 3 for n in 1..5}
 for w in ["pear", "apple"]
 for k in [w, w ++ "!"] if ends_with(k, "!")}
```

An empty comprehension retains general dictionary kind when its key facts are
non-string or deferred, so `lookup({[x]: 1 for x in []}, [1])` returns absence.
Known string-key comprehensions keep the existing string dictionary representation.

The dictionary iterates as `pear!`, then `apple!`; each value is the set
`1, 2, 0`. A final collection inside an explicit control or function block
keeps its own braces: `if true { {3, 1, 3} } else { set<int>() }` returns
`3, 1` as a set.

```zirql
sort({"z": 2, "a": 2, "m": 1})
sort_by({"z": 2, "a": 2, "m": 1}, value)
sort_by({"z": {"cost": 2}, "a": {"cost": 2}, "m": {"cost": 1}}, "cost", descending=true)
```

The first dictionary has key order `a, m, z`. The second has order `m, z, a`.
The last has order `z, a, m`: descending comparisons retain the current order
of the tied values. Bare `value` selects a dictionary value; quoted `"value"`
selects that field within each value. Dictionary key sorting permits
heterogeneous values because it does not inspect them as ordering keys.
Unsupported or absent selector keys fail at the selector.

Sorting, copying and saving a dictionary retain its kind and current order.
Dictionary equality compares key/value associations independently of entry
order. JSON objects require string keys; general dictionaries need a host-chosen
encoding or an explicit serialization error. Hosts rendering string dictionaries
as JSON objects preserve their member order.
Keyed arrays remain distinct: they retain typed axes, missing cells and
row-major cell order.

Sets use the callable `Set` node or set comprehensions. For example,
`ValueExpressionKind::Set` with integer expressions `3, 1, 3, 2` yields a set
ordered `3, 1, 2`; wrapping it in `Sort` yields a set ordered `1, 2, 3`.
Typed empty sets retain their supplied element identity through sorting.

The common reader accepts bare-brace sets, computed-key dictionaries, typed
empty general or scalar sets and indexed brackets with optional `axes(...)` domains.
The Rust API exposes the same constructors. Declared sources run before cells
or generators and retain unused keys; `KeyedComprehension::empty_axis_sources`
remains a lazy fallback that runs only when no cell is produced. All
construction, comparison, copies and rendering use the active budget and retain
expression locations and native owners.

## Explicit scalar conversions

Use [`parse_int`, `parse_float` and `to_string`](library-strings.md#parse_int)
when crossing between strings and primitive scalar values. Conversion checks the
written operand's known type before evaluation, parses only evaluated strings,
and reports located errors for malformed or out-of-range text. It adds no implicit
numeric coercion, native stringification or CSV inference.

The [collection selection reference](library-pipelines.md#filter) defines filter,
bounds, reversal and checks for arrays, sets, dictionaries and admitted native
collections. Selection preserves declared empty schemas; removed structural
record fields become optional while retaining their value contracts.
