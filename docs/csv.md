# Query CSV files with Zirql

The [CSV example](../examples/csv.rs) reads each file as a table of string
fields, using its first row as column names. You can filter rows, select fields
and combine named files in a report. To build an application like this one, follow
[Build the CSV host](csv-host.md).

These commands run from the root of a `v0.1.2` checkout or the extracted
`zirql-0.1.2` crate with Rust 1.88 or later. The
[setup guide](getting-started.md#run-the-example) describes both. This is a Cargo
example executable; installing the library does not install a CSV command.

Use the tabs to compare a query, its input and its result, or copy a command.
The commands pass a multiline query as one quoted shell argument. Results below
are indented for reading; the executable writes compact JSON.

## Select rows from one file

Select active people and keep their names and notes. Switch to **CSV input** to
see the exact source, including CSV quoting and a cell containing a newline.

<!-- tabs:start -->
### Query

```zirql
let selected = rows | filter(_.active == "yes");
[
  {"name": p.name, "note": p.note}
  for p in selected
]
```

### CSV input

[examples/csv/people.csv](../examples/csv/people.csv):

```csv
id,name,team,active,note
2,Grace Hopper,compiler,yes,"Said ""hello"""
1,"Ada, Lovelace",compiler,yes,"First line
second line"
3,Barbara Liskov,runtime,yes,
4,Edsger Dijkstra,compiler,no,01
```

### Result

```json
[
  {"name": "Grace Hopper", "note": "Said \"hello\""},
  {"name": "Ada, Lovelace", "note": "First line\nsecond line"},
  {"name": "Barbara Liskov", "note": ""}
]
```

### Run

```sh
cargo run --locked --example csv -- '
let selected = rows | filter(_.active == "yes");
[
  {"name": p.name, "note": p.note}
  for p in selected
]
' \
  examples/csv/people.csv
```
<!-- tabs:end -->

`rows` names the sole input table. Filtering retains file order. CSV quoting
preserves the comma in Ada's name, the quote in Grace's note and the newline in
Ada's note. JSON escapes that newline inside its string. Empty cells stay empty
strings. All cells are strings, including `id`, `yes`, `no` and `01`; the host
does not guess types or trim labels and values.

`filter(p => p.active == "yes")` names each row; `filter(_.active == "yes")`
uses the unnamed row argument. Both select the same rows as
`[p for p in rows if p.active == "yes"]`. Predicates can capture earlier bindings:

```zirql
let wanted = "yes";
rows | filter(_.active == wanted)
```

To count the rows, pipe the table into `count`:

```sh
cargo run --locked --example csv -- \
  'rows | count' \
  examples/csv/people.csv
```

This prints `4`. The shorter `count` expression uses the same table implicitly.
To count only active people, reuse the saved selection:

```sh
cargo run --locked --example csv -- '
let selected = rows | filter(_.active == "yes");
selected | count
' \
  examples/csv/people.csv
```

This prints `3`.

## Use your own CSV file

Replace the fixture path with your file. Counting rows needs no column names:

```sh
cargo run --locked --example csv -- \
  'rows | count' \
  './my data.csv'
```

Use `rows` as the query to inspect the decoded records. Then adapt a selection
by changing its field names and string comparisons to match your headers and
values. Quote paths containing spaces; for a named input, quote the whole
argument, such as `'people=./my data.csv'`.

The input must be comma-separated UTF-8 with a header row. Headers must be
nonempty and unique, and every data row must have the header's field count.
The example accepts at most 64 KiB across all input files and at most 1,000 data
rows per table. These are application limits; the
[host tutorial](csv-host.md#7-evaluate-under-one-budget) explains where to change
them.

An identifier header supports access such as `p.name`. For a header containing
spaces, use `p["full name"]` or `lookup(p, "full name")`. A column name can also
come from a binding:

```sh
cargo run --locked --example csv -- '
let column = "name";
[p[column] for p in rows if p.active == "yes"]
' \
  examples/csv/people.csv
```

This returns `["Grace Hopper", "Ada, Lovelace", "Barbara Liskov"]`. Brackets
read the computed string key; `p.name` selects a fixed label. An unknown label
fails because the CSV header declares the available columns. Strict field
access is checked even when a table has no data rows.

Cells remain strings, so sorting an amount column sorts its text: `"10"` comes
before `"2"`. An empty cell is `""`, not `null`. If your application needs
numeric columns or another treatment of empty cells, change the loader and its
type declarations as described in [Build the CSV host](csv-host.md).

## Reuse selections across named files

Name each input with `name=path`. Save the active people, build a member list for
each team, then use those saved lists for both sorted names and counts.

<!-- tabs:start -->
### Query

```zirql
let selected = people | filter(p => p.active == "yes");
let groups = [
  {
    "team": t.label,
    "members": [p.name for p in selected if p.team == t.id]
  }
  for t in teams
];
[
  {
    "team": g.team,
    "members": sort(g.members),
    "count": length(g.members)
  }
  for g in groups
]
```

### Inputs

[people.csv](../examples/csv/people.csv), with its decoded cell values:

| id | name | team | active | note |
| --- | --- | --- | --- | --- |
| 2 | Grace Hopper | compiler | yes | `Said "hello"` |
| 1 | Ada, Lovelace | compiler | yes | First line<br />second line |
| 3 | Barbara Liskov | runtime | yes | |
| 4 | Edsger Dijkstra | compiler | no | 01 |

[teams.csv](../examples/csv/teams.csv):

| id | label |
| --- | --- |
| runtime | Runtime |
| compiler | Compiler |
| operations | Operations |

### Result

```json
[
  {
    "team": "Runtime",
    "members": ["Barbara Liskov"],
    "count": 1
  },
  {
    "team": "Compiler",
    "members": ["Ada, Lovelace", "Grace Hopper"],
    "count": 2
  },
  {"team": "Operations", "members": [], "count": 0}
]
```

### Run

```sh
cargo run --locked --example csv -- '
let selected = people | filter(p => p.active == "yes");
let groups = [
  {
    "team": t.label,
    "members": [p.name for p in selected if p.team == t.id]
  }
  for t in teams
];
[
  {
    "team": g.team,
    "members": sort(g.members),
    "count": length(g.members)
  }
  for g in groups
]
' \
  people=examples/csv/people.csv \
  teams=examples/csv/teams.csv
```
<!-- tabs:end -->

The outer comprehension follows team-file order. Its inner comprehension sees
the current team and the saved `selected` value. `sort` puts Ada before Grace
within the Compiler list, while `length` counts its original members. Operations
has no active members and still appears with an empty list. JSON keeps both
array order and the written `team`, `members`, `count` field order.

The member field can also start a pipeline: `g.members | sort` gives the same
result as `sort(g.members)`.

Bindings are immutable and visible after their initializer. A `let` cannot
replace a table name, `rows`, or an earlier binding. Comprehension names stay
inside their expression. The example accepts ordered `let` bindings followed by
one final expression; named function declarations are unsupported.

Use exactly one unnamed file, or only named files. Named tables use ASCII
identifiers: a letter or underscore followed by letters, digits or underscores.
`_`, `rows`, `input`, literals and control/operator keywords are reserved. Names
must be unique. One named table also supplies `rows` and the implicit current
input. With several tables, use their names explicitly; neither `rows` nor
`count` chooses a file for you.

## Build a dictionary report

When each team needs one count, use its label as a dictionary key. This example
uses the same people and teams files as the previous report.

<!-- tabs:start -->
### Query

```zirql
let selected = people | filter(p => p.active == "yes");
sort_by(
  {
    t.label: length([p for p in selected if p.team == t.id])
    for t in teams
  },
  value,
  descending=true
)
```

### Result

```json
{
  "Compiler": 2,
  "Runtime": 1,
  "Operations": 0
}
```

### Run

```sh
cargo run --locked --example csv -- '
let selected = people | filter(p => p.active == "yes");
sort_by(
  {
    t.label: length([p for p in selected if p.team == t.id])
    for t in teams
  },
  value,
  descending=true
)
' \
  people=examples/csv/people.csv \
  teams=examples/csv/teams.csv
```
<!-- tabs:end -->

The inner comprehension counts selected people in the current team. Every team
gets an entry, including Operations with no active members. `sort_by` orders the
counts from largest to smallest and retains the dictionary kind. Equal counts
would keep their incoming team order. Duplicate labels fail because a
dictionary requires distinct keys.

## Use patterns and other expression forms

Patterns can extract fields in a predicate or a `let` binding. Select the first
active person, then extract their name:

```sh
cargo run --locked --example csv -- '
let [first, ..] = rows | filter(
  {active: status, ..} => status == "yes"
);
let {name, ..} = first;
name
' \
  examples/csv/people.csv
```

This returns `"Grace Hopper"`. The record pattern binds the `active` field to
`status`; `..` ignores the other fields. The list pattern requires at least one
selected row. If none exists, the let fails at `[first, ..]` and stdout remains
empty.

Both comprehension spellings use the same reader and evaluator:

```sh
cargo run --locked --example csv -- \
  '[p.name for p in rows if p.active == "yes"]' \
  examples/csv/people.csv

cargo run --locked --example csv -- \
  '[name | {name, active, ..} in rows where active == "yes"]' \
  examples/csv/people.csv
```

Both return the three active names in file order. The same patterns and lexical
rules apply to named inputs.

Nested filters can refer to an outer argument. To keep only teams with active
people:

```sh
cargo run --locked --example csv -- '
teams | filter(t =>
  length(people | filter(_.team == t.id and _.active == "yes")) > 0
)
' \
  people=examples/csv/people.csv \
  teams=examples/csv/teams.csv
```

```json
[
  {"id": "runtime", "label": "Runtime"},
  {"id": "compiler", "label": "Compiler"}
]
```

The outer filter binds `t` to a team. Inside the inner filter, `_` is a person
and `t` remains available for the comparison. Operations is omitted because its
selection is empty.

See [values and comprehensions](values.md) for expression syntax and
[the language reference](language-reference.md) for collection stages,
conditionals and indexed literals.

## Inspect an indexed value

The reader can also construct values addressed by integer keys. The
[saved indexed example](../examples/csv/indexed-arrays.zirql) retains keys
`5, 2, 9`, with cells at `5` and `2`:

```sh
cargo run --locked --example csv -- \
  "$(cat examples/csv/indexed-arrays.zirql)" \
  examples/csv/people.csv
```

```json
{
  "keys": [5, 2, 9],
  "cells": [
    {"key": 5, "value": 50},
    {"key": 2, "value": 20}
  ],
  "second": 20,
  "has_nine": false
}
```

`index_set` keeps unused keys; `entries` visits only present cells. The CSV
renderer accepts those ordinary arrays and records. Direct keyed values retain
their shape in the library and cannot be rendered as plain JSON by this host.
See [keyed arrays](values.md#keyed-arrays) for row and column domains, typed
empties and sparse cells.

## Understand errors

A missing column fails at its original query range:

```sh
cargo run --locked --example csv -- \
  'rows | filter(_.missing == "yes")' \
  examples/csv/people.csv
```

Several named tables make an implicit request ambiguous:

```sh
cargo run --locked --example csv -- \
  'count' \
  people=examples/csv/people.csv \
  teams=examples/csv/teams.csv
```

A finite range can exceed the request's result allowance before it is built:

```sh
cargo run --locked --example csv -- \
  '1..100001' \
  examples/csv/people.csv
```

Each command exits unsuccessfully with empty stdout. Query diagnostics include
the submitted text, byte range and available table paths. CSV failures identify
the table and path, with the decoder's record, line and byte position where
available. Malformed query syntax also retains its original byte range. A failed
request returns no partial JSON.

## How the example evaluates a query

The example loads at most 64 KiB of CSV source bytes across all input files,
using bounded reads. It supplies constructed arrays and records, header types,
named-table lookup and final JSON rendering. Input loading, decoder admission,
copies, parsing, evaluation and output share one budget: 1,000,000 work units,
100,000 produced or copied results, nesting depth 64 and a 1,000-item stream
ceiling. Work includes scanned and copied bytes, field metadata and output
bytes. The byte cap bounds the CSV decoder's temporary buffers as well.

Zirql's common readers parse the expressions and `let` prefixes. The host lowers
the bindings to one value block and calls `evaluate_query_value` once. Zirql
therefore owns scope, saved-value reuse, correlated iteration, expressions,
collection stages and their resource failures. The output writer walks the final
constructed value directly, retaining record order. It renders absence as JSON
`null` and tuples and sets as arrays. General dictionaries use tagged entries:
`{"kind":"dictionary","entries":[{"key":...,"value":...}]}`. Each key and value
is encoded recursively; string dictionaries remain ordinary JSON objects.
Nominal members and keyed arrays are refused because this encoding would
discard their identity or shape.

[Build the CSV host](csv-host.md) walks through the Rust implementation, from
loading a table to returning JSON. The [host integration guide](host-integration.md)
explains the adapter traits; [embedding programs](embedding-programs.md) covers
the larger native-program API.
