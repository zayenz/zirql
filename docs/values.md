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
| Integer | `42`, `-3` | Signed 64-bit, with checked arithmetic |
| Float | `1.5`, `2.0e3` | Finite binary64 values |
| String | `"apple"` | Double-quoted text |
| Absence | `null`, `<>` | One absent value with two accepted spellings |
| Array | `["ash", "apple"]` | Ordered; duplicates and different element types are allowed |
| Tuple | `("ash", 3)` | Fixed positions; distinct from an array |
| Dictionary | `{"word": "ash", "count": 1}` | String keys in insertion order |
| Set | `{3, 1, 3}` | Homogeneous values; first occurrence wins |

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

## Operators and precedence

The common reader applies these precedence levels, strongest first:

| Level | Operations |
| --- | --- |
| Access | `a[i]`, `tuple.1`, `row.field` |
| Optional fallback | `default` |
| Prefix | `not`, unary `+` and `-` |
| Multiplication and division | `*`, `/`, `div`, `mod` |
| Addition and subtraction | `+`, `-` |
| Inclusive integer range | `..` |
| String concatenation | `++` |
| Ordering | `<`, `<=`, `>`, `>=` |
| Equality | `==` (also `=`), `!=` |
| Boolean conjunction | `and` |
| Boolean disjunction | `or` |

Binary operators at the same level associate to the left. Use parentheses to
make a fallback boundary clear: `(lookup(row, "count") default 0) + 1`.
Parentheses also group expressions; a comma makes a tuple.

Whitespace separates tokens, and `#` starts a line comment outside a string.
A host may use other surface syntax while preserving the shared value rules.

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

This returns `"ash"`. With empty input it returns `"no words"` without trying
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

`lookup(container, key)` returns absence for a valid missing entry. Record keys
are strings, positional keys are one-based integers, and keyed arrays retain
their declared key types and rank. Wrong key types or incompatible containers
fail. For a record with a declared schema, a label outside that schema also fails.

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

Later generators can use earlier names. Each source is evaluated before its
own binder becomes available; its filter can use that binder. The rightmost
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

## Array summaries and requirements

| Call | Meaning | Empty input |
| --- | --- | --- |
| `length(a)` | Number of present array entries | `0` |
| `sum(a)` | Checked integer sum in input order | `0` |
| `forall(a)` | Every entry is true | `true` |
| `exists(a)` | At least one entry is true | `false` |

These calls accept positional and keyed arrays. Keyed summaries visit present
cells in stored order; they do not fill missing cells. `sum` requires integers,
and `forall` and `exists` require known Booleans. None accepts a string, tuple or
set as its array operand.

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

`sort(a)` orders an array or set by its scalar values and a dictionary by its
string keys. It retains the collection kind. `sort_by(rows, "field")` orders
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

Ordering keys must be present and homogeneous: integers, finite floats, strings
or members of one nominal type. Integers and floats do not mix. Boolean sets also support their usual false-before-true order. Array and
dictionary Boolean keys and native handles need a supported projection first. A missing field, absent
key or later incompatible key fails the whole operation, including an extremum
that already has a candidate winner. Project an explicit default key before
sorting if missing data should have an order. Keyed arrays need an explicit
conversion such as `[cell.value for cell in entries(a)]` before ordering.

## Constructed collection pipelines

A positional string array can pass through `filter(starts_with("prefix"))`.
It retains matches in order, including duplicates. The prefix must be a literal
string; matching is case-sensitive and an empty prefix matches every string.
`count` returns the number of array entries as an integer. For example:

```zirql
{"beginning_with_a": filter(starts_with("a")) | count, "total": count}
```

With the word host this returns `{"beginning_with_a": 3, "total": 4}`. Each
field reads the same immutable constructed input. An explicit source, such as
`["apple", "birch"] | filter(starts_with("a")) | count`, supplies only its own
pipeline and returns `1`. Empty and no-match arrays count as zero. Missing
input, scalar or keyed filter sources and non-string elements fail at their
query locations. Incompatible declarations fail even for empty arrays.

The `sort` suffix applies the same ascending sort to arrays, sets and
dictionaries, retaining their kind. Positional arrays also support stable
`unique` and terminal string `tally`:

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

Tally accepts only strings and returns `{"ash": 1, "apple": 2}`, with record
fields in first-seen order and checked signed integer counts. No further
stage may follow tally. Empty untyped arrays yield `[]` for sort/unique and
`{}` for tally. Explicit incompatible element declarations still fail on empty
input; unique retains valid declarations and element kinds.

A pipe applies to the completed expression, after arithmetic, Boolean and
`default` operators. A completed comprehension or conditional can pipe directly.
Group a bare constructed binding or user function call explicitly:

```zirql
(words) | sort | unique
(make_words()) | tally
```

The second form requires a host that registers `make_words`. Bare names and
user calls followed by a pipe retain their established native routing. Group
any pipeline result before using it in a larger expression, for example
`length(([w for w in words] | unique))`. Count produces a scalar; a subsequent
collection stage fails at that stage. The richer `sort(a, descending=true)` and
`sort_by` call forms keep their existing contracts.

## Nominal members and sets

A nominal type gives its members an identity as well as a name. A host might use
one for a fixed set of word categories. Identity includes both the type and
member, so identical printed names from different types are not equal. Members
use declaration order. Arithmetic on members and mixed nominal types fail.

Typed sets remove equal elements while retaining each first occurrence,
including the sign of the first floating zero. Supported scalar set elements
are Booleans, integers, finite floats and nominal members. Strings are not set
elements. Set equality compares membership independently of stored order.
`sort` orders a set explicitly by its supported type order and returns a set.
Bare braces construct sets: `{3, 1, 3}` iterates as `3, 1`, and
`{n mod 3 for n in 1..5}` iterates as `1, 2, 0`. Empty braces mean a dictionary;
use `set<int>()`, `set<bool>()` or `set<float>()` for an empty scalar set.
A host can supply a typed empty nominal set through the Rust API. The common
reader does not resolve nominal type names.

## Keyed arrays

A keyed array has typed ordered axes and cells addressed by explicit keys.
It is distinct from a positional or nested array. Keys `2` and `5` do not imply
that cells `3` and `4` exist. A higher-rank cell has one key component per axis.

Zirql permits sparse arrays, where some combinations of axis keys have no cell.
Full keys are unique and stored in axis order, with the rightmost dimension
varying fastest. Empty results still need typed axes. Array access checks rank
and key types. A host may impose additional rules, such as contiguous axes or
complete rectangular coverage.

The host's parser supplies keyed construction syntax. The shared Rust
representation also supports keyed comprehensions. See
[value evaluation](value-runtime.md#keyed-construction) for those boundaries.

`index_set(a)` returns a typed set of one-dimensional array keys. For a
positional array this is `1..length(a)`, including a typed empty integer set
for `[]`. For a rank-one keyed array it uses the declared axis, including keys
with no stored cell, and retains integer or nominal identity for empty axes.
The result retains the declared axis order, even when it differs from numeric
or nominal declaration order. Inputs of another kind or rank fail at the call.

```zirql
[words[i] for i in index_set(words)]
```

This returns the words in positional order. The common reader accepts
`index_set`; keyed constructors remain supplied by the host.

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
reads each `w` as its key. Dictionaries use `ConstructedValue::Record`;
wholly static quoted fields retain their structural schemas. `.field` selects a static label; bracket access and `lookup`
accept computed string keys. Empty `{}` is an empty dictionary.

Dictionaries preserve entry insertion order. Each entry evaluates its key,
requires a present string, checks for a duplicate, then evaluates its value.
A duplicate fails before that value runs. The callable `construct_string_entries` operation
also supports computed keys, evaluates each key before its value and returns
`Vec<(String, V)>` in insertion order. Keys must be strings; there is no
implicit conversion from integer, member or native-handle text.

Repeated `for` clauses and filters use the same scope and traversal rules as
array comprehensions:

```zirql
{k: {n mod 3 for n in 1..5}
 for w in ["pear", "apple"]
 for k in [w, w ++ "!"] if ends_with(k, "!")}
```

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
Dictionary equality compares key/value associations independently of field
order. Hosts rendering a JSON object must preserve the same member order.
Keyed arrays remain distinct: they retain typed axes, missing cells and
row-major cell order.

Sets use the callable `Set` node or set comprehensions. For example,
`ValueExpressionKind::Set` with integer expressions `3, 1, 3, 2` yields a set
ordered `3, 1, 2`; wrapping it in `Sort` yields a set ordered `1, 2, 3`.
Typed empty sets retain their supplied element identity through sorting.

The common reader does not yet accept bare-brace sets, computed-key brace
expressions, typed-empty set text or indexed-axis grammar. Hosts can construct
those supported values directly through the Rust API. The current keyed
comprehension `empty_axis_sources` supplies axes only when no cell is produced.
All construction, comparison, copies and rendering use the active budget and
retain expression locations and native owners.
