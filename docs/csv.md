# Query CSV files with Zirql

The [CSV example](../examples/csv.rs) reads header-named string fields and runs
Zirql expressions over their rows. It accepts one file as the current input, or
several named files for a correlated report. These commands run from the
root of a `v0.1.0` checkout or the extracted `zirql-0.1.0` crate with Rust 1.88
or later. The [setup guide](getting-started.md#run-the-example) describes both.

## Select rows from one file

The [people fixture](../examples/csv/people.csv) has `id`, `name`, `team`, `active`
and `note` columns. Select the active people and keep their names and notes:

```sh
cargo run --locked --example csv -- 'let selected = rows | filter(_.active == "yes"); [{"name": p.name, "note": p.note} for p in selected]' examples/csv/people.csv
```

```json
[{"name":"Grace Hopper","note":"Said \"hello\""},{"name":"Ada, Lovelace","note":"First line\nsecond line"},{"name":"Barbara Liskov","note":""}]
```

CSV quoting preserves the comma in Ada's name, the quote in Grace's note and the
newline in Ada's note. JSON escapes that newline inside its string. Empty cells
stay empty strings. All cells are strings, including `id`, `yes`, `no` and `01`;
the host does not guess types or trim labels and values.

`rows` names the sole input table. Pipe it into `count` to count the rows:

```sh
cargo run --locked --example csv -- 'rows | count' examples/csv/people.csv
```

```json
4
```

The shorter `count` expression uses the same table implicitly.

A saved selection can feed a shared collection stage directly:

```sh
cargo run --locked --example csv -- 'let selected = rows | filter(_.active == "yes"); selected | count' examples/csv/people.csv
```

```json
3
```

`filter(p => p.active == "yes")` names each row; `filter(_.active == "yes")`
uses the unnamed row argument. Both agree with
`[p for p in rows if p.active == "yes"]`. Predicates can capture earlier bindings,
such as `let wanted = "yes"; rows | filter(_.active == wanted)`.

See [values and comprehensions](values.md) for expression syntax and
[the language reference](language-reference.md) for the shared collection stages.

## Reuse selections across named files

Name each input with `name=path`. The [teams fixture](../examples/csv/teams.csv)
lists Runtime, Compiler and Operations in that order. This program saves active
people, builds a member list for each team, then uses those saved lists twice:

```sh
cargo run --locked --example csv -- '
let selected = people | filter(p => p.active == "yes");
let groups = [{"team": t.label, "members": [p.name for p in selected if p.team == t.id]} for t in teams];
[{"team": g.team, "members": sort(g.members), "count": length(g.members)} for g in groups]' people=examples/csv/people.csv teams=examples/csv/teams.csv
```

```json
[{"team":"Runtime","members":["Barbara Liskov"],"count":1},{"team":"Compiler","members":["Ada, Lovelace","Grace Hopper"],"count":2},{"team":"Operations","members":[],"count":0}]
```

The outer comprehension follows team-file order. Its inner comprehension sees
the current team and the saved `selected` value. `sort` puts Ada before Grace
within the Compiler list, while `length` counts its original members. Operations has no active members and
still appears with an empty list. JSON keeps both array order and the written
`team`, `members`, `count` field order.

Bindings are immutable and visible after their initializer. A `let` cannot
replace a table name, `rows`, or an earlier binding. Comprehension names stay
inside their expression. The example accepts ordered `let name = expression;`
bindings followed by one final expression; named function declarations are not
part of its input format.

Use exactly one unnamed file, or only named files. Named tables use ASCII
identifiers: a letter or underscore followed by letters, digits or underscores.
`_`, `rows`, `input`, literals and control/operator keywords are reserved. Names
must be unique. One named table also supplies `rows` and the implicit current
input. With several tables, use their names explicitly; neither `rows` nor
`count` chooses a file for you.

Headers must be nonempty, unique UTF-8 strings. Every data row must have the
header's field count. Identifier headers support strict access such as `p.name`.
For a CSV with a `full name` header, use `lookup(p, "full name")`. Unknown
header labels are outside the declared schema and fail. Header declarations
also check strict column access when the table has no data rows.

## Inspect an indexed value

The same reader can construct values addressed by integer keys. The
[saved indexed example](../examples/csv/indexed-arrays.zirql) retains keys
`5, 2, 9`, with cells at `5` and `2`:

```sh
cargo run --locked --example csv -- "$(cat examples/csv/indexed-arrays.zirql)" examples/csv/people.csv
```

```json
{"keys":[5,2,9],"cells":[{"key":5,"value":50},{"key":2,"value":20}],"second":20,"has_nine":false}
```

`index_set` keeps unused keys; `entries` visits only present cells. The CSV
renderer accepts those ordinary arrays and records. Direct keyed values retain
their shape in the library and cannot be rendered as plain JSON by this host.
See [keyed arrays](values.md#keyed-arrays) for row and column domains, typed
empties and sparse cells.

## Build a dictionary report

When each team needs one count, use its label as a dictionary key:

```sh
cargo run --locked --example csv -- '
let selected = people | filter(p => p.active == "yes");
sort_by({t.label: length([p for p in selected if p.team == t.id]) for t in teams}, value, descending=true)
' people=examples/csv/people.csv teams=examples/csv/teams.csv
```

```json
{"Compiler":2,"Runtime":1,"Operations":0}
```

The inner comprehension counts selected people in the current team. Every team
gets an entry, including Operations with no active members. `sort_by` orders the
counts from largest to smallest and retains the dictionary kind. Equal counts
would keep their incoming team order. Duplicate labels fail because a
dictionary requires distinct string keys.

A column name can also come from an expression:

```sh
cargo run --locked --example csv -- 'let column = "name"; [p[column] for p in rows if p.active == "yes"]' examples/csv/people.csv
```

This returns `["Grace Hopper","Ada, Lovelace","Barbara Liskov"]`, in input
order. Brackets read the computed string key; `p.name` selects a fixed label.

## Read failures

A missing column fails at its original query range:

```sh
cargo run --locked --example csv -- 'rows | filter(_.missing == "yes")' examples/csv/people.csv
```

Several named tables make an implicit request ambiguous:

```sh
cargo run --locked --example csv -- 'count' people=examples/csv/people.csv teams=examples/csv/teams.csv
```

A finite range can exceed the request's result allowance before it is built:

```sh
cargo run --locked --example csv -- '1..100001' examples/csv/people.csv
```

Each command exits unsuccessfully with empty stdout. Query diagnostics include
the submitted text, byte range and available table paths. CSV failures identify
the table and path, with the decoder's record, line and byte position where
available. A failed request returns no partial JSON.

## The host boundary

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
`null`, tuples and sets as arrays, and refuses nominal members or keyed arrays
whose identity or shape ordinary JSON would discard.

The [host integration guide](host-integration.md) explains the adapter traits;
[embedding programs](embedding-programs.md) covers the larger native-program API.
The CSV example needs only the value-reader and value-query interfaces.
