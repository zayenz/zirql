# Language reference

A Zirql expression transforms data into a result. That data may be a string
array, a record, or a selection of native objects supplied by an application.
The [word tutorial](examples.md) starts with an array and follows it through
selection, adjacent pairs and grouped occurrences. This reference gives the
rules behind those examples.

The application embedding Zirql is its host. Strings, arrays and records are
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
arrays, tuples and records. Nominal members, typed sets and keyed arrays retain
additional type or index information. Native objects keep their host's identity.

```zirql
42
"apple"
["ash", "apple", "birch", "apple"]
{"word": "apple", "count": 2}
```

Arrays and tuples use one-based positions. `words[1]` is `"ash"` in the word
example; `(2, 5).2` selects the second tuple value. Record access uses a label,
such as `row.word`. Ordinary access fails
when an entry is missing; `lookup(row, "word")` returns absence for a valid
missing key.

Arithmetic is checked. Integer overflow and division by zero fail, and strings
are not coerced to numbers. `++` concatenates strings. Boolean `and` and `or`
short-circuit. A conditional evaluates only the chosen branch:

```zirql
if length(words) > 0 { words[1] } else { "no words" }
```

All branches still receive scope and known-type checks. See
[values and comprehensions](values.md) for exact arithmetic, collection,
absence and access rules.

## Comprehensions

A comprehension constructs a collection from a source and an optional filter:

```zirql
[{"word": w} for w in words if starts_with(w, "a")]
```

The head constructs each result. The generator binds `w`, and the condition
selects which words reach the head. Arrays preserve generation order and
repeated values. Multiple `for` clauses form a nested traversal; later sources
can use earlier binders.

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
{"beginning_with_a": filter(starts_with("a")) | count, "total": count}
["ash", "apple", "birch", "apple"] | filter(starts_with("a")) | sort | unique | count
[w for w in words if starts_with(w, "a")] | sort | unique
[w for w in words if starts_with(w, "a")] | tally
```

The first two return `{"beginning_with_a": 3, "total": 4}` and `2` with the
word host. Bare filter/count use the same constructed input independently in
each record field; an explicit source supplies its own stages. The filter
accepts only `starts_with` with a literal string prefix and requires positional
strings. Empty and no-match inputs count as zero. Count returns a scalar, so a
later collection stage is invalid. Missing input and incompatible types receive
located errors. The last two return `["apple", "ash"]` and
`{"ash": 1, "apple": 2}`. Pipes apply
left-to-right after ordinary operators, defaults, comprehensions and
conditionals. Tally is terminal. Group bare constructed names and user calls,
such as `(words) | unique` or `(make_words()) | tally`, to preserve the native
routing of an ungrouped name or call. See [collection pipelines](values.md#constructed-collection-pipelines)
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

Callable sets and dictionaries preserve insertion order and retain their kind
through explicit sorting. `sort(d)` orders dictionary keys; `sort_by(d, value)`
orders values and retains current-order ties. The common reader accepts static
string dictionaries; set construction is available through Rust nodes and
comprehensions. See [collection order](values.md#selected-collection-syntax-and-ordering)
for current syntax and examples. Native adapters adopt the ordered dictionary
APIs separately; their current pinned versions retain their documented behavior.

Use `project(subject, {fields})` to turn native subjects into report records.
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

Patterns can destructure tuples and records, bind names and test optional
presence. A native capture match enumerates correlated domain objects; a value
match selects one result. Their forms and results are different.

The [functions and control flow chapter](functions-and-control-flow.md)
describes matching, guarded arms and local bindings. It also distinguishes the
syntax accepted by the common parser from control-flow forms available through
the Rust API.

## Errors and limits

A missing field, incompatible type, failed requirement or exhausted allowance
is an error with a location. An absent value is ordinary data. An unknown native
predicate is a third truth state, used when the host lacks a fact; negation
preserves that uncertainty.

The application chooses finite limits for parsing and execution. Nested queries,
comprehensions and host callbacks share the active budget. See
[diagnostics and limits](diagnostics-and-limits.md) for the contract.
