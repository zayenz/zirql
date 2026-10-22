# Build the CSV host

The CSV example loads files into declared sequences of records, binds those
values by name, then asks Zirql to read and run a program. Its last step renders
the completed value into a JSON or Markdown buffer. The application owns file
access and CSV rules; Zirql owns the common program machinery.

The [complete implementation](https://github.com/zayenz/zirql/blob/v0.3.0/examples/csv.rs)
is a runnable starting point. The [CSV guide](csv.md) shows its queries and
results. Use Zirql 0.3.0 and Rust 1.88 or newer.

## 1. Start with a runnable application

From the Zirql checkout, create a sibling application and copy the example and
fixtures:

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

Change the usage message in `main` to name your application. The copied tests
use the fixture paths under `examples/csv`. `csv` belongs in ordinary application
dependencies; Zirql's manifest lists it as a development dependency because it
serves an example. `serde_json` is used only by the copied tests.

```sh
cargo run -- '
let selected = rows | filter(_.active == "yes");
[p.name for p in selected]
' examples/csv/people.csv
```

This prints `["Grace Hopper","Ada, Lovelace","Barbara Liskov"]`. The query is
the first argument. The remaining arguments select the input files, optional
`--format json|markdown`, and repeated `--param name=value` inputs.

## 2. Load exact cells

`InputSpec`, `load_table` and `decode_table` belong to the application. The loader
reads bounded chunks and enforces a 64 KiB aggregate limit across files. The CSV
decoder handles quoting, embedded newlines and UTF-8. Headers must be nonempty
and unique, and each record must have the header's field count.

Each cell becomes `ConstructedValue::String`, preserving its decoded contents.
An empty cell is an empty string. A quantity such as `02` remains `"02"`.
No trimming or numeric inference occurs. Each row becomes a `Record` in header
order, and the table becomes an `Array` in file order.

Charge file scans, string bytes, labels and containers before allocating them.
The example checks row and column counts against the request's stream allowance.
Decoder errors retain their record, line and byte position where available.

## 3. Declare and bind the table

Build one declaration from the headers: each field has `DeclaredType::String`.
After decoding, give the value and declaration to `ConstructedBinding::new`:

```rust
let table = ConstructedBinding::new(
    V::Array(rows),
    DeclaredType::Sequence(Box::new(DeclaredType::Record(declaration))),
    budget,
    header_location,
)?;
```

A sequence declaration establishes positional storage and its element schema,
even for a header-only file. Zirql checks actual values against that declaration
and derives compact preparation facts from it. The application does not maintain
another header schema or one type entry per row. A query that reads an unknown
column is refused even when there are no rows to visit.

The binding retains an immutable shared owner. Reusing it as a named table,
`rows`, and current input shares that owner. It does not allocate another table.
Copies produced by an operation still spend the active request allowance.
`ConstructedBinding::value()` borrows the admitted data for application inspection.

## 4. Select names and current input

The example accepts exactly one unnamed file or one or more named files.
`admit_inputs` checks external table names with `is_unquoted_binding_name` and
`is_common_keyword`, excludes `rows`, and rejects duplicates. For example,
`people=people.csv` is a named input; `for=people.csv` is refused.

With one file, the application supplies the same binding as `rows` and current
input. A sole named file also retains its chosen name. Both `rows | count` and
bare `count` then count its rows.

With several named files, only their explicit names have values. `rows` and
implicit operations fail with a located diagnostic listing the available table
names. This table-selection rule stays in CSV's request boundary. The common
constructed API simply accepts an optional current input.

## 5. Read, prepare and run

The request follows this order:

```text
admit input arguments
  → load and declare tables
  → read the common program
  → admit parameters and prepare functions/statements
  → evaluate the final value
  → render a complete buffer
```

Create one `Budget` before loading data. Use it through reading, parameter
admission, preparation, evaluation and rendering. The CSV limits are 1,000,000
work units, 100,000 result units, nesting 64, and 1,000 stream items.
The separate input byte limit bounds decoder buffers before values exist.

The public call sequence is small:

```rust
let program = ConstructedProgram::read(query, &names, &mut budget, limits.nesting)?;
let result = program.evaluate(
    &named,
    current,
    ParameterInputs::Cli(parameters),
    FunctionLimits { active_depth: 32 },
    &mut budget,
)?;
```

`names` contains the initial binding names. `named` borrows `(name, binding)`
pairs, and `current` is an optional borrowed binding. `parameters` contains raw
CLI strings collected by repeated `--param`; it is separate from `name=FILE`
arguments. Library callers can use `ParameterInputs::Typed` with exact scalar
values. Zirql's `admit_parameters` handles required, duplicate, unknown and
mistyped values without substituting text into the query.

`ConstructedProgram` uses `read_program`, borrowed function signatures,
`PreparedFunctions` and `PreparedProgram`. Those components own forward function
names, parameter positions, ordered lets, destructuring, scope and execution.
The example implements no native reader or value-query traits.

A program contains functions, typed `param` declarations, pattern lets and one
final value expression. Lets and parameters require semicolons; the final value
has none. Parameters become visible at their written declarations. Functions
receive all data explicitly: their bodies cannot capture tables, program
parameters or current input. Existing pattern parameters, recursive calls and
active-depth limits use the common function runtime. Failures retain query
locations and bounded function-call frames.

The [parameterized orders report](csv.md#calculate-with-numeric-cells) passes its
table and threshold into a named function and runs with either implicit or
explicitly named input.

## 6. Render the completed value

Rendering is an explicit step after successful evaluation:

```rust
let location = Location::Query(0..query.len());
let mut output = render_json(
    &result,
    JsonRepresentation::Ordinary,
    JsonLayout::Compact,
    &mut budget,
    location.clone(),
)?;
budget.charge(1, location.clone())?;
budget.charge_results(1, location)?;
output.push(b'\n');
```

For Markdown, use `render_markdown(&result, &mut budget, location)`. Both
renderers charge conversion, table expansion and escaped bytes to the remaining
request budget. See [JSON and Markdown rendering](library-rendering.md) for the
representations and supported table layouts.

`run` returns the complete `Vec<u8>`. `main` writes it only after success.
Parsing, parameter admission, conversion, evaluation or rendering failure
therefore leaves stdout empty, with diagnostics on stderr. An I/O failure while
writing a completed buffer remains possible.

## Adapt the loader

For another CSV file, pass its path; the headers define its fields. For a new
input source, replace the file and decoding functions and supply the value with
its declaration. An application with data already in memory can start directly
at `ConstructedBinding::new`. The [minimal Rust example](embedding-programs.md#constructed-data-programs)
shows that path without CSV.

Keep decoded CSV columns string-valued when following this example. Queries use
[`parse_int` and `parse_float`](library-strings.md#parse_int) for explicit numeric
work. [F-strings and `format`](library-strings.md#format) construct labels from
those scalars; the [orders report](csv.md#calculate-with-numeric-cells) formats
prices and totals to two decimal places. A loader that chooses typed columns
must construct those actual values, declare their types and report malformed
cells at their source positions.

For native nodes, model objects, source coordinates or domain operations, use
the [native embedding interfaces](host-integration.md). Those interfaces retain
native handles and domain admission while using the same common language and
evaluator.
