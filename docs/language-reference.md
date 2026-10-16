# Language reference

A Zirql expression transforms data into a result. That data may be a string
array, a dictionary, or a selection of native objects supplied by an application.
The [word tutorial](examples.md) starts with an array and follows it through
selection, adjacent pairs and grouped occurrences. This reference gives the
rules behind those examples.

The application embedding Zirql is its host. Strings, arrays and dictionaries are
constructed values, which Zirql can inspect directly. Native values identify
objects kept by the host, such as words in a source document or nodes in a
syntax tree. A projection reads properties of those objects into constructed
values.

The available data and domain operations depend on the application. The word
example accepts a single expression; applications can also support complete
programs with parameters and named functions. Each chapter identifies where
host support is needed. For the version used by these examples, see the
[setup guide](getting-started.md#use-zirql-in-another-rust-project).

## Values and expressions

Values include Booleans, signed integers, finite floats, strings, absence,
arrays, tuples and dictionaries. Nominal members, typed sets and keyed arrays retain
additional type or index information. Native objects keep their host's identity.

```zirql
42
"apple"
["ash", "apple", "birch", "apple"]
{"word": "apple", "count": 2}
```

Arrays and tuples use one-based positions. `words[1]` is `"ash"` in the word
example; `(2, 5).2` selects the second tuple value. Dictionary access uses a string key,
such as `row.word` or `row["word"]`. Ordinary access fails
when an entry is missing; `lookup(row, "word")` returns absence for a valid
missing key.

Arithmetic is checked. Integer overflow and division by zero fail, and strings
are not coerced to numbers. `++` concatenates strings. Boolean `and` and `or`
short-circuit. `/\` and `\/` spell the same Boolean operators. A conditional
evaluates only the chosen branch:

```zirql
if length(words) > 0 { words[1] } else { "no words" }
```

The same conditional can use `if length(words) > 0 then words[1] else "no words" endif`,
with `elseif condition then value` for additional branches. All branches still
receive scope and known-type checks. See
[values and comprehensions](values.md) for exact arithmetic, collection,
absence and access rules.

Names may be single-quoted, such as `'row count'`. Quotes are omitted from
name identity while diagnostics retain the original bytes; `'if'` reads a name.
Strings use double quotes and JSON escapes. Whitespace and `#`/`%` line comments
or nonnested `/* ... */` comments separate tokens. Integers also accept `0x`/`0X`
hexadecimal and `0o` octal forms within signed 64-bit bounds. Hexadecimal floats
are unsupported.

Natural records such as `(word: "apple", count: 2)` have the same ordered string
fields as the dictionary example above. Indexed values accept `a[i, j]` as
`a[(i, j)]`; `[| 1, 2 | 3, 4 |]` constructs a rank-two indexed matrix, and
`[3: 7, 8]` gives its cells keys `3` and `4`. See [keyed arrays](values.md#keyed-arrays)
for explicit keys and empty shapes.

## Save intermediate results

A `let` gives a calculated value an immutable name. Later expressions can reuse
it. This complete report filters sample counts, then sorts a dictionary by value:

```sh
cargo run --locked --example csv -- '
let samples = [
  {"label": "pear", "count": 2},
  {"label": "apple", "count": 1},
  {"label": "ash", "count": 2},
  {"label": "birch", "count": 3}
];
let selected = [row for row in samples if row.count > 1];
sort_by({row.label: row.count for row in selected}, value, descending=true)
' examples/csv/people.csv
```

```json
{"birch":3,"pear":2,"ash":2}
```

`selected` contains pear, ash and birch in their input order. The dictionary uses
their labels as string keys and their counts as values. `descending=true` puts
the largest count first. Pear and ash both have count 2, so their incoming order
stays pear, then ash. Sorting retains the dictionary kind.

The CSV file supplies the example application's required input; this calculation
uses only `samples`. The [CSV walkthrough](csv.md) applies the same operations to
file contents. [Zirium](https://zayenz.se/zirium/guides/query-language/) and
[Zincite](https://zayenz.se/zincite/guides/queries/) run the same calculation in
their own programs, alongside operations on MLIR and MiniZinc source.

Bindings are visible after their initializer. They cannot replace an existing
name, refer forward or refer to themselves. A comprehension's iteration names
stay inside that expression. The host decides which statement and output forms
its frontend accepts; CSV accepts ordered lets and one final expression.

## Comprehensions

A comprehension constructs a collection from a source and an optional filter:

```zirql
[{"word": w} for w in words if starts_with(w, "a")]
```

The head constructs each result. The generator binds `w`, and the condition
selects which words reach the head. Arrays preserve generation order and
repeated values. Multiple `for` clauses form a nested traversal; later sources
can use earlier binders.

The MiniZinc spelling uses a generator pipe and `where`:

```zirql
[{"word": w} | w in words where starts_with(w, "a")]
```

Both forms use the same patterns, scope and evaluator. The head sees every
binder. Each source sees only earlier generators; its filter also sees the
current generator's names. Grouped binders such as `x, y in [1, 2]` form a
Cartesian product, with the rightmost binder varying fastest. Commas separate
MiniZinc generators. That spelling also supports a single-value binding:
`[total | x in [1, 2], total = x * 2 where total > 2]` returns `[4]`.
The common spelling requires `for pattern in source`.

Parentheses protect a native pipeline in a head or source. A pipe after the
closing collection starts its suffix, as in `[x * 2 | x in [1, 2, 3] where x > 1] | count`.


Bare braces construct a set without an outer colon, or a string dictionary
with one:

```zirql
{n mod 3 for n in 1..5}
{w: length([w]) for w in ["pear", "apple"]}
```

The set keeps `1, 2, 0`; the dictionary keeps `pear`, then `apple`.
Keys are expressions and must produce present strings. Duplicate keys fail
before their values run. Empty `{}` is a dictionary. Empty scalar sets use
`set<int>()`, `set<bool>()` or `set<float>()`.

Indexed brackets retain integer or nominal row and column keys:

```zirql
[2: 20, 5: 50] axes([5, 2, 9])
[(i, j): j for i in [2, 5] for j in [8, 1]] axes([5, 2, 9], [8, 1, 4])
[] axes(set<int>(), [8, 1])
```

The optional `axes(...)` suffix declares complete ordered key domains, including
keys without cells. Its sources run in surrounding scope before the contents.
Without it, nonempty keys establish their dimensions. Empty `[]` is positional;
a keyed empty result needs established domains. See [keyed arrays](values.md#keyed-arrays).

Comprehensions are eager. A reducer such as `length` or `sum` receives the
completed array, and rejected candidates still cost work. See
[positional comprehensions](values.md#positional-comprehensions) and
[keyed arrays](values.md#keyed-arrays) for construction and index rules.

## Built-in operations

These operations work on constructed values. Native stages can have similar
names but operate on a host's native selections.

| Operation | Meaning |
| --- | --- |
| `contains(s, part)`, `starts_with(s, prefix)`, `ends_with(s, suffix)` | Exact, case-sensitive string matching |
| `length(a)`, `sum(a)` | Count present array entries or sum integer entries |
| `forall(a)`, `exists(a)` | Reduce an array of known Booleans |
| `sort(a)`, `sort_by(d, value)`, `sort_by(rows, "field")` | Stable same-kind array, set or dictionary ordering, optionally `descending=true` |
| `min(a)`, `max(a)`, `min_by(rows, "field")`, `max_by(rows, "field")` | Select an extremum, or absence on empty input |
| `group_by(rows, "field")` | First-seen groups, each with `key` and `items` |
| `lookup(a, key)`, `has_key(a, key)` | Optional access and membership |
| `occurs(v)`, `absent(v)` | Test presence or absence |
| `v default fallback` | Evaluate the fallback only when `v` is absent |
| `one(a)` | Require exactly one element |
| `require(condition, value, "message")` | Return the value on true; otherwise fail with the message |
| `index_set(a)` | Return typed keys of a positional or rank-one keyed array |
| `entries(a)`, `zip_exact(a, b)` | Inspect keyed cells or pair arrays with identical axes and present keys |

The [value reference](values.md) defines accepted types, empty inputs and error
cases. For example, `length` accepts arrays, not strings, and `sum` uses checked
integer addition. `forall` and `exists` inspect an eager Boolean array; they do
not turn its construction into a lazy traversal.

## Constructed pipelines

The common reader accepts constructed filter/count and collection stages:

```zirql
{"beginning_with_a": filter(starts_with(_, "a")) | count, "total": count}
["ash", "apple", "birch", "apple"] | filter(starts_with(_, "a")) | sort | unique | count
[w for w in words if starts_with(w, "a")] | sort | unique
[w for w in words if starts_with(w, "a")] | tally
```

The first two return `{"beginning_with_a": 3, "total": 4}` and `2` with the
word host. Bare filter/count use the same constructed input independently in
each dictionary field; an explicit source supplies its own stages. The filter
accepts `pattern => Boolean_expression` or an expression using `_`.
The subject is one positional element; named and unnamed forms agree with
comprehension selection. For example, `people | filter(_.active == "yes")` and
`people | filter(p => p.active == "yes")` retain the same rows. Nested predicates
rebind `_`; explicit outer names retain lexical captures. Destructured arguments
still give `_` the whole subject. `_` is unavailable outside a predicate argument
and keeps its wildcard meaning in patterns. Empty and no-match inputs count as
zero. Count returns a scalar, so a
later collection stage is invalid. Missing input and incompatible types receive
located errors. The last two return `["apple", "ash"]` and
`{"ash": 1, "apple": 2}`. Pipes apply
left-to-right after ordinary operators, defaults, comprehensions and
conditionals. Tally is terminal. Known constructed bindings can pipe directly,
such as `words | unique`. Grouping also works, including for a user call such
as `(make_words()) | tally`. Hosts retain native routing for native sources. See [collection pipelines](values.md#constructed-collection-pipelines)
for supported scalar kinds, empty results and grouping in larger expressions.

## Pipelines and native data

For native data, a pipeline applies stages in written order. A host exposing
borrowed words can support:

```zirql
filter(prefix("a")) | head(1) | count
```

Here `prefix` is a host predicate that tests a word's spelling. It is separate
from the built-in `starts_with` function used with constructed strings. Other
hosts can define predicates over syntax nodes or document handles. Zirql
supplies the shared sequencing and Boolean composition.

[Pipeline semantics](shared-concepts.md) cover filtering, ordering, identity,
terminal results and the small `parse_pipeline` reader. The broader native
representation also supports host stages and set operations. A host can reuse
the common readers or lower an existing parser to that representation.

Sets and dictionaries preserve insertion order and retain their kind through
explicit sorting. `sort(d)` orders dictionary keys; `sort_by(d, value)` orders
values and retains current-order ties. Bare braces accept sets and dictionaries,
including computed string keys. See [collection order](values.md#selected-collection-syntax-and-ordering)
for syntax and examples. Both native hosts retain dictionary order through
saved results and JSON rendering.

Use `project(subject, {fields})` to turn native subjects into report dictionaries.
Explicit projections produce constructed values; native captures and paths can
retain the original objects. See [native programs](native-programs.md) for
projection, `match { from ...; bind ...; where ...; }` and bounded paths.

## Bindings, parameters and functions

Programs execute statements in order and save values in immutable bindings. A
name must be available before use. Required external parameters have `int`,
`string` or `bool` types and are checked before domain execution.

The common reader recognizes `let name =`, `param name: type` and `report`
prefixes. A host connects them to its program and emission policy. Single
expressions, including every front-page example, need no declarations.

Named pure functions have typed parameters and a result type:

```zirql
fn twice(n: int) -> int { n * 2 }
```

Functions can call themselves or each other recursively. They can inspect
native data through registered pure operations, but cannot emit output or edit
the source. See [function declarations](functions-and-control-flow.md#declare-a-function)
for parameter types, scope and recursion limits.

## Patterns and control flow

Value matching selects the first matching arm and requires a final unguarded
wildcard:

```zirql
match (length(words)) {
  0 => "empty",
  1 => "one word",
  _ => "several words"
}
```

Patterns destructure tuples, positional lists and records, bind names and test
optional presence. `[a, b]` requires exactly two list elements; `[a, b, ..]`
discards a remainder after at least two. `[]` requires empty and `[..]` accepts
any positional list. Tuples keep exact tuple arity; keyed arrays and sets do not
match list patterns. Lets, typed parameters, predicate arguments and comprehension
binders use these same patterns. Their mismatches are located errors; value-match
arms try the next pattern. A native capture match enumerates correlated domain
objects; a value match selects one result. Their forms and results are different.

The [functions and control flow chapter](functions-and-control-flow.md)
describes matching, guarded arms, local bindings, loops and named reducers.
The common reader supports these forms; each host connects them to its program
and native-type handling.

## Errors and limits

A missing field, incompatible type, failed requirement or exhausted allowance
is an error with a location. An absent value is ordinary data. An unknown native
predicate is a third truth state, used when the host lacks a fact; negation
preserves that uncertainty.

The application chooses finite limits for parsing and execution. Nested queries,
comprehensions and host callbacks share the active budget. See
[diagnostics and limits](diagnostics-and-limits.md) for the contract.
