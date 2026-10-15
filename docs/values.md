# Values and comprehensions

A value expression computes a scalar or builds a collection. The word model
supplies `words = ["birch", "apple", "ash", "apple"]`; the examples on this page
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
| Record | `{"word": "ash", "count": 1}` | Unique labels in written order |

Use a field label or a one-based position to select a value:

```zirql
{"word": "ash", "count": 1}.word
```

The result is `"ash"`. `words[1]` is `"birch"`, and `(2, 5)[2]` is `5`. Position
zero, a position beyond the collection and a missing field are errors. Use
`lookup` when a missing entry is expected.

Strings are scalar values. Array operations such as `length` do not implicitly
operate on their characters. A host can expose a byte count or another text
measure when its model needs one.

## Operators and precedence

The common reader applies these precedence levels, strongest first:

| Level | Operations |
| --- | --- |
| Access | `a[i]`, `row.field` |
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

This returns `"birch"`. With empty input it returns `"no words"` without trying
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

It returns records for `apple`, `ash`, `apple`, in that order. The `++` operator
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

The result is `"birch"`. On empty input the requirement fails before the access.
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

`sort(a)` orders a positional array of scalars. `sort_by(rows, "field")` orders
records by a literal field label. Both are ascending and stable.
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
or members of one nominal type. Integers and floats do not mix. Boolean values
and native handles need a supported projection first. A missing field, absent
key or later incompatible key fails the whole operation, including an extremum
that already has a candidate winner. Project an explicit default key before
sorting if missing data should have an order. Keyed arrays need an explicit
conversion such as `[cell.value for cell in entries(a)]` before ordering.

## Nominal members and sets

A nominal type gives its members an identity as well as a name. A host might use
one for a fixed set of word categories. Identity includes both the type and
member, so identical printed names from different types are not equal. Members
use declaration order. Arithmetic on members and mixed nominal types fail.

Typed sets remove equal elements and use their type's order. Supported scalar
set elements are Booleans, integers, finite floats and nominal members. Strings
are not set elements. Set construction has a Rust representation; its text
spelling belongs to the frontend.

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
representation also supports keyed comprehensions and rank-one index-set
extraction. See [value evaluation](value-runtime.md#keyed-construction) for
those boundaries.

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
