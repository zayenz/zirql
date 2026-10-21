# Build the CSV host

The CSV host turns files into arrays of records and makes them available to
Zirql by name. Zirql reads and evaluates the query, then its shared renderer
converts the result to JSON or Markdown. This tutorial follows that implementation
so you can adapt it to your own application. The [CSV guide](csv.md) shows what
its users can query.

The complete implementation is [examples/csv.rs](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs). Code
blocks below are excerpts from that file, with the surrounding responsibilities
explained at each step. Use the complete file for a runnable application; the
excerpts omit imports and some error and budget handling.

## 1. Start with a runnable application

Use Rust 1.88 or newer and this working Zirql checkout. The shared rendering
APIs are newer than published Zirql 0.2.0. With the checkout directory named
`zirql`, create a sibling application and copy the matching example and fixtures:

```sh
cargo new --bin --edition 2024 ../csv-query
cp examples/csv.rs ../csv-query/src/main.rs
mkdir -p ../csv-query/examples
cp -R examples/csv ../csv-query/examples/csv
cd ../csv-query
cargo add zirql --path ../zirql
cargo add csv@1.4
cargo add --dev serde_json@1
```

In the copied `src/main.rs`, change the usage message in `main` to name your
application:

```rust
eprintln!("usage: csv-query 'QUERY' [--format json|markdown] FILE.csv | name=FILE.csv ...");
```

The original message names the repository's Cargo example. The copied test
module also uses the fixture paths under `examples/csv`, so keep that directory
if you keep the tests.

The application's dependencies are:

```toml
[dependencies]
zirql = { path = "../zirql" }
csv = "1.4"

[dev-dependencies]
serde_json = "1"
```

In Zirql's own manifest, `csv` is a development dependency because it serves an
example. Your application needs it as an ordinary dependency. `csv` decodes
quoted records. Zirql reads and evaluates the query language and renders JSON or
Markdown. The copied tests use `serde_json` to inspect results; the application itself
does not call it.

Run a selection through your new application:

```sh
cargo run -- '
let selected = rows | filter(_.active == "yes");
[p.name for p in selected]
' \
  examples/csv/people.csv
```

It prints `["Grace Hopper","Ada, Lovelace","Barbara Liskov"]`. The application
accepts its query as the first argument, followed by one file or named files:

```sh
cargo run -- \
  '{"people": length(people), "teams": length(teams)}' \
  people=examples/csv/people.csv \
  teams=examples/csv/teams.csv
```

This prints `{"people":4,"teams":3}`. The rest of the tutorial explains how to
adapt this working application to another input source.

## 2. Choose the value representation

A CSV table fits an array of records. Each record stores header/value pairs in
header order, and each cell is a string:

```rust
use std::sync::Arc;
use zirql::ConstructedValue as V;

struct Table {
    headers: Vec<String>,
    rows: Arc<V>,
}
```

For example, the first row of `people.csv` becomes a record with `id = "2"`,
`name = "Grace Hopper"`, `team = "compiler"`, `active = "yes"` and
`note = "Said \"hello\""`. Empty cells become `V::String(String::new())`.
Numeric-looking text keeps its exact spelling. The host retains the immutable
table in an `Arc`, which lets the evaluator share its ownership without copying
all rows when it first reads a binding. Each invocation owns its own tables.

This representation gives Zirql access to ordinary fields and collection
operations. CSV rows do not need native handles, custom predicates or a native
pipeline evaluator. Choose native values when queries must retain objects from
an existing model or navigate domain relationships; see
[choosing a data model](implementation-reference.md#choose-a-data-model).

## 3. Load and declare each table

The [load_table and decode_table functions](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L277)
read bounded chunks, enforce an aggregate 64 KiB limit across files, then decode
the bytes. The CSV decoder handles quoting, embedded newlines and UTF-8. The
loader requires a nonempty header, unique nonempty labels and the same field
count in each record. It does not trim or infer cell types.

The inner decoding loop constructs each record after charging for its fields:

```rust
budget.charge(size, location.clone())?;
fields.push((header.clone(), V::String(cell.to_owned())));
```

The row and table containers are then built with `V::Record(fields)` and
`V::Array(rows)`. The complete loop also checks row counts, charges result
allocations and checks the field-size sum before allocating.

Attach the header declaration to the table's stored value:

```rust
let rows = declare_constructed(
    V::Array(rows),
    DeclaredType::Array(Box::new(DeclaredType::Record(declaration))),
    budget,
    header_location.clone(),
)?;
budget.charge(1, header_location.clone())?;
budget.charge_results(1, header_location)?;
Ok(Table {
    headers,
    rows: Arc::new(rows),
})
```

Here `declaration` contains one `(header, DeclaredType::String)` pair per column.
Keep the declaration even for a header-only file. A table without data still
has known columns. Charge the shared owner allocation before wrapping the
declared value in `Arc::new`.

The evaluator also needs type facts before it reads binding values. The
[Table::types method](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L259) derives those facts from headers:

```rust
impl Table {
    fn types(
        &self,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Vec<ValueType>, QueryError> {
        budget.charge(self.headers.len() + 3, location.clone())?;
        let mut fields = Vec::new();
        let mut declaration = Vec::new();
        for header in &self.headers {
            budget.charge(header.len() * 2 + 2, location.clone())?;
            fields.push((header.clone(), ValueType::String));
            declaration.push((header.clone(), DeclaredType::String));
        }
        let row = ValueType::Declared {
            actual: Box::new(ValueType::Record(fields)),
            declaration: DeclaredType::Record(declaration),
        };
        Ok(vec![ValueType::Array(ArrayType::Elements(vec![row]))])
    }
}
```

The header schema makes `p.missing` an error even if a comprehension has no
rows to visit. Both the stored declaration and the type facts describe the same
array of string records. For another data source, change them together.

## 4. Expose names to the reader

The [CsvHost](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L404) retains loaded tables, their input names
and the names introduced by parsed lets:

```rust
struct CsvHost<'a> {
    tables: Vec<Table>,
    inputs: &'a [InputSpec],
    let_names: Vec<String>,
}
```

`InputSpec` stores an optional name and a path. `admit_inputs` checks table
names with `is_unquoted_binding_name` and `is_common_keyword`, then excludes
the host's `rows` alias. This external-name context rejects `for` and `input`,
while `lookup` remains a valid table name. It also rejects duplicates or a
mixture of named and unnamed inputs. With one table,
`implicit` returns it; with several, it returns a diagnostic asking the user to
choose a named table.

Implement `ValueReaderHost` so the shared reader recognizes these bindings:

```rust
fn field_pipeline_source(&self) -> zirql::FieldPipelineSource {
    zirql::FieldPipelineSource::Constructed
}

fn is_binding(&self, name: &str) -> bool {
    name == "rows"
        || self
            .inputs
            .iter()
            .any(|input| input.name.as_deref() == Some(name))
        || self.let_names.iter().any(|binding| binding == name)
}

fn is_constructed_binding(&self, name: &str) -> bool {
    self.is_binding(name)
}
```

`FieldPipelineSource::Constructed` lets a field such as `g.members` feed
shared collection stages, as in `g.members | sort`. The same reader owns field
access, comparisons, predicates, comprehensions and sorting.

The remaining [reader methods](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L437) declare the host's
unsupported native operations. `Stage`, `SetOperator`, `Emitter` and `Predicate`
are `()`. `is_stage` returns `false`, `set_operator` returns `None`, and the
native stage and predicate parsers return located errors. The required native
`filter` and `projection` constructors have empty bodies because this reader
never admits those native forms. Shared constructed filters and projections
continue to work.

## 5. Supply values and current input

Implement [ValueQueryHost](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L481) for
`ReadExpression<Self>`, with `type Error = QueryError`. This trait connects
binding names to their data and defines the optional implicit input.

The `binding` helper resolves `rows` through `implicit`; other names select the
corresponding table in `inputs`. The shared hook returns a reference-counted
owner of that table:

```rust
fn read_shared_binding(
    &mut self,
    name: &str,
    range: &Range<usize>,
    budget: &mut Budget,
) -> Result<Option<Arc<V>>, QueryError> {
    let table = self.binding(name, range)?;
    // Share the retained owner; Zirql still admits the full table value.
    budget.charge(1, Location::Query(range.clone()))?;
    Ok(Some(Arc::clone(&table.rows)))
}
```

This path serves named bindings such as `people` and the implicit table alias
`rows`. Cloning the `Arc` shares the table; it does not copy its contents.
Zirql still performs full runtime admission when it first reads the binding in
this evaluation, checking the table under the request's active budget. Sharing
removes the initial owned copy while retaining those checks.

Keep `read_binding` as the owned fallback. It uses the same `binding` helper and
returns `copy_constructed_value(&table.rows, budget, location)`, charging for
the copy. Hosts without a retained immutable owner can leave
`read_shared_binding` at its default `Ok(None)` to use that owned path.
Zirql resolves saved `let` bindings internally; these hooks supply only table
data.

The two current-input hooks use the same table and schema:

```rust
fn current_input_types(
    &mut self,
    range: &Range<usize>,
    budget: &mut Budget,
) -> Result<Vec<ValueType>, QueryError> {
    self.implicit(range)?
        .types(budget, Location::Query(range.clone()))
}

fn read_current_input(
    &mut self,
    range: &Range<usize>,
    budget: &mut Budget,
) -> Result<V, QueryError> {
    copy_constructed_value(
        &self.implicit(range)?.rows,
        budget,
        Location::Query(range.clone()),
    )
}
```

A shorthand query such as `count` reads current input through the owned
`read_current_input` hook. The existing API has no shared current-input hook.
Write `rows | count` to use the shared binding path. Both forms return the same
count and keep copies or sharing within the same request allowance.

The other hooks have small, explicit roles:

| Hook | CSV implementation |
| --- | --- |
| `validate_name` | Reject ambiguous `rows` access before execution |
| `query_types` | Return an error for native query leaves |
| `evaluate_query` | Return an error for native query execution |

## 6. Read lets and a final expression

Follow [read_program](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L558). Start an
`ArgumentReader::new(query, 0)` so diagnostics retain byte offsets in the user's
original query. Repeatedly:

1. Call `read_pattern_let` to read a let pattern, or stop when there is none.
2. Call `read_value_expression` for its initializer.
3. Use `pattern_names` to add the newly bound names to `host.let_names` after
   reading the initializer. Later expressions can then reference them.
4. Require `;` and retain the `(pattern, value)` pair.

Read one final expression with `read_value_expression` and reject trailing text.
When there are bindings, wrap them and the final value in one located block:

```rust
Located::new(
    K::Block {
        bindings,
        value: Box::new(value),
    },
    0..query.len(),
)
```

The source imports `ValueExpressionKind as K`. With no bindings, return the
final expression directly. Pass the same budget and nesting allowance to every
reader call. On parsing failures, release retained patterns with
`dispose_pattern` and expression trees with `dispose_native_value`, as the
complete function does.

The host reads the small program envelope. Zirql evaluates binding scope,
destructuring and each initializer once, including saved selections used by
nested comprehensions. There is no separate CSV expression evaluator. This
frontend admits ordered lets and one expression; named function declarations
need the [larger program interfaces](embedding-programs.md).

## 7. Evaluate under one budget

The [run function](https://github.com/zayenz/zirql/blob/feature/extensions/examples/csv.rs#L94) shows the complete request order:

```text
admit input arguments
  → load tables and their header declarations
  → build binding type facts
  → read the query
  → evaluate the value
  → render into a buffer
  → return the completed bytes
```

Create one budget at the beginning:

```rust
let mut budget = Budget::for_program(limits);
```

Use it for input loading, type facts, parsing, owner allocation and sharing,
value copies, evaluation and rendering. In `run`, the `types` map contains each
named table and `rows`. With one input, `rows` has that table's schema. With several inputs, it has
`ValueType::Unestablished`; the host refuses actual implicit access through
`validate_name` and the current-input hooks.

The evaluation and cleanup sequence is:

```rust
let expression =
    read_program(query, &mut host, &mut budget, limits.nesting)
        .map_err(query_error)?;
let result = evaluate_query_value(&expression, &types, &mut host, &mut budget);
dispose_native_value(expression);
let result = result.map_err(query_error)?;
```

Here `query_error` adds the submitted query and table paths to a `QueryError`.
Disposing the expression before propagating the evaluation error handles both
success and failure. The returned constructed value belongs to the caller.

The example chooses these limits:

```rust
const LIMITS: ProgramLimits = ProgramLimits {
    work: 1_000_000,
    results: 100_000,
    nesting: 64,
    stream_items: 1_000,
};
```

Choose limits that fit your application. `stream_items` also limits the loader
to 1,000 rows per table and 1,000 columns per header. The separate
`MAX_INPUT_BYTES` constant caps source bytes at 64 KiB across all files, bounding
decoder buffers before Zirql values exist.

The core charges its own operations; the host charges its file reads, metadata,
allocations and copies. The shared renderers charge conversion, table expansion
and output.
[Budget and diagnostic APIs](runtime-limits.md) describes that accounting.

## 8. Render only a completed result

Call a shared [renderer](library-rendering.md) after evaluation. The JSON branch is:

```rust
let location = Location::Query(0..query.len());
let mut output = render_json(
    &result,
    JsonRepresentation::Ordinary,
    JsonLayout::Compact,
    &mut budget,
    location.clone(),
)
.map_err(query_error)?;
budget.charge(1, location.clone()).map_err(query_error)?;
budget.charge_results(1, location).map_err(query_error)?;
output.push(b'\n');
```

Import `render_json`, `JsonRepresentation` and `JsonLayout` from `zirql`.
Ordinary output keeps record fields and dictionary entries in stored order.
Absence becomes `null`; tuples and sets become arrays. General dictionaries use
recursive key/value entry pairs. Nominal members and keyed arrays use the
explicit tagged representation, including declared axes and present cells.
The [direct keyed-data command](csv.md#inspect-an-indexed-value) returns an
unused declared key as well as the two present cells.

Zirql checks constructed data, applies the active nesting/reentry guard and
charges conversion, copied text and escaped output bytes. Rendering returns a
complete buffer or the original located failure, including its budget category.
The JSON branch adds its framing newline under the same budget.

For `--format markdown`, call `render_markdown(&result, &mut budget, location)`
instead. Import it from `zirql`. It returns a complete buffer with its own blank
line framing. The same constructed rows become a table; nested cells use compact
shared JSON. Table expansion and escaped bytes spend the remaining request
budget, and errors retain the supplied query range. The small `output_arguments`
function selects `json` or `markdown` after the positional query while preserving
both positional and named file inputs.

`run` builds all output in a `Vec<u8>`, including its framing. `main`
writes that buffer only after the request succeeds. Parsing, evaluation and
rendering failures leave stdout empty; diagnostics go to stderr. An I/O error
while writing the completed buffer is still possible.

CSV diagnostics retain a decoder record, line and byte position where available.
Query diagnostics use `Location::Query` ranges from the shared reader. Keep
these input coordinates when adapting the example so errors can identify the
offending query text or input record.

## Adapt it to your own input

To query another CSV file with the same rules, pass its path; headers become
fields automatically. No Rust changes are needed. For a different source of
tabular data, replace `InputSpec`, `load_table` and `decode_table`, then update
`Table::types` and the stored declaration to match your fields. Keep the reader
and value-query interfaces while your data fits
constructed arrays and records. If your application already has typed data in
memory, start at the value construction step and omit CSV decoding.

For typed CSV columns, make conversion a loader decision. For example, parse an
`amount` column as an integer and store `V::Integer`. Use `DeclaredType::Integer`
for that column in both `decode_table` and `Table::types`; the latter also needs
`ValueType::Integer` for its value facts. Report a bad cell at its CSV record
position. Decide how empty cells behave before changing their representation;
the example preserves them as empty strings.

Decide which names users can reference, whether one source can be implicit and
how callers receive the final value. A library API can return the constructed
value directly. A command-line tool can select either shared buffered renderer.

Reuse Zirql's grammar and evaluator for filters, saved values, correlated
iteration and collection operations. Add native interfaces when your domain
needs retained handles, identity or navigation. The
[host integration guide](host-integration.md) describes those obligations.
