# JSON and Markdown rendering

Zirql renders constructed results after evaluation. The caller chooses JSON or
Markdown, passes the request's remaining budget, and receives a complete buffer.
These Rust APIs belong to the working checkout; published Zirql 0.2.0 does not include them. They are not query-language calls.

## to_json_value

`to_json_value(value, representation, budget, location)` borrows a
`ConstructedValue` and returns `Result<serde_json::Value, QueryError>`. It checks
publicly constructed data, including finite floats, unique record labels and
valid keyed shape. Declared wrappers are transparent.

`JsonRepresentation::Ordinary` uses these encodings:

| Zirql value | JSON |
| --- | --- |
| Absent | `null` |
| Boolean, integer, finite float, string | Corresponding scalar |
| Positional array, tuple, set | Array in stored order |
| Record | Object in stored field order |
| General dictionary | `{"kind":"dictionary","entries":[{"key":...,"value":...}]}` |
| Nominal member, keyed array | Their tagged encoding below |

Dictionary keys and values are converted recursively in insertion order. A
structural key remains an array, object or tagged dictionary; it is never
stringified. General dictionaries always use entry pairs, even when their keys
happen to be strings. Records supply ordinary string-key objects.

`JsonRepresentation::Tagged` uses explicit kinds:

| Zirql value | JSON |
| --- | --- |
| Absent | `{"kind":"absent"}` |
| Boolean, integer, float, string | `{"kind":"boolean","value":true}` and corresponding kind/value |
| Nominal member | `{"kind":"member","enum":"Color","name":"Blue"}` |
| Positional array | `kind: "array"`, `indexing: "positional"`, one complete one-based axis and keyed entries |
| Keyed array | `kind: "array"`, `indexing: "explicit"`, all declared axes and present keyed entries |
| Tuple, set | `{"kind":"tuple","elements":[...]}` or `kind: "set"` |
| Record | Object with tagged field values |
| General dictionary | Tagged entry pairs with recursively tagged keys and values |

Array axes and entry keys are arrays of tagged integers or members. Ordinary
output also uses tagged children inside an explicit keyed array. For example,
`[2: <>] axes([2, 9])` becomes:

```json
{
  "kind": "array",
  "indexing": "explicit",
  "axes": [[{"kind": "integer", "value": 2}, {"kind": "integer", "value": 9}]],
  "entries": [
    {"key": [{"kind": "integer", "value": 2}], "value": {"kind": "absent"}}
  ]
}
```

Key `2` has a present absent cell; key `9` has no cell. Empty axes remain in
`axes`. Supplied axis order, dimension order and present-cell order survive.
Nominal tags retain type and member names, without inventing portable IDs or
an importer that reconstructs native identity.

## Mixed native reports

`to_report_json_value(host, value, budget, location)` traverses a borrowed
report through `RenderHost`. `RenderView` distinguishes constructed values,
sequences, records, captures, transparent retained values and projected native
leaves. The host selects ordinary or tagged JSON for each constructed node.
Native descriptors keep their ordinary shape. A mixed report therefore needs no
second recursive serializer in the adapter.

This complete Rust example renders one native descriptor beside tagged data:

```rust
use zirql::{
    Budget, ConstructedValue, JsonRepresentation, Location, ProgramLimits,
    RenderContext, RenderHost, RenderView, to_report_json_value,
};

enum Report {
    Record(Vec<(String, Report)>),
    Data(ConstructedValue),
    Operation,
}
struct Host;
impl RenderHost for Host {
    type Value = Report;
    type Error = zirql::QueryError;

    fn view<'a>(
        &mut self, value: &'a Report, _: RenderContext,
        budget: &mut Budget, location: &Location,
    ) -> Result<RenderView<'a, Report>, Self::Error> {
        Ok(match value {
            Report::Record(fields) => RenderView::Record(fields),
            Report::Data(value) => RenderView::Constructed(value, JsonRepresentation::Tagged),
            Report::Operation => {
                // This sample has no native owner; real handles need owner checks.
                budget.charge(13, location.clone())?;
                budget.charge_results(13, location.clone())?;
                RenderView::Projected(serde_json::json!({"name": "native"}))
            }
        })
    }
}
let report = Report::Record(vec![
    ("subject".into(), Report::Operation),
    ("count".into(), Report::Data(ConstructedValue::Integer(3))),
]);
let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 10_000, nesting: 64, stream_items: 100,
});
let json = to_report_json_value(&mut Host, &report, &mut budget, Location::Query(0..5))?;
assert_eq!(json["count"]["kind"], "integer");
# Ok::<(), zirql::EvaluationError<zirql::QueryError>>(())
```

A host checks native ownership and refuses queues or identity sets in `view`.
It admits native descriptor allocation before returning `Projected`; Zirql
admits the borrowed containers, copied labels and constructed data. Captures
preserve field and row order. `RenderContext::CaptureField` lets Zirium retain
its singleton operation descriptor inside a capture without changing ordinary
operation sequences. This bridge keeps constructed nodes typed for other
renderers; it is separate from evaluation and session traits.

Zincite uses tagged constructed nodes inside ordinary native containers and
retains its `complete`, `limitations` and `result` envelope. Zirium uses ordinary
constructed nodes, with tagged fallback for members and keyed arrays, and keeps
its document and diff envelopes. Both use `render_json_value` for data bytes.
Their native MiniZinc and MLIR source renderers remain host operations.

## render_json

`render_json(value, representation, layout, budget, location)` converts the
constructed value and returns `Result<Vec<u8>, QueryError>`. Choose
`JsonLayout::Compact` or `JsonLayout::Pretty`. Both produce UTF-8 JSON with the
same content and object/entry order. Neither adds a framing newline.

```rust
use zirql::{
    Budget, ConstructedValue as V, JsonLayout, JsonRepresentation, Location,
    ProgramLimits, render_json,
};

let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 10_000, nesting: 64, stream_items: 100,
});
let result = V::Record(vec![("count".into(), V::Integer(3))]);
let bytes = render_json(
    &result, JsonRepresentation::Ordinary, JsonLayout::Compact,
    &mut budget, Location::Query(0..5),
)?;
assert_eq!(bytes, br#"{"count":3}"#);
```

The [CSV executable](csv.md#inspect-an-indexed-value) uses this call after
evaluation, then charges and appends its newline before returning the buffer.

## render_json_value

`render_json_value(value, layout, budget, location)` borrows an existing
`serde_json::Value` and returns a complete `Vec<u8>`. A host can use
`to_json_value` for the data, construct its own document or completeness
envelope, then render that JSON tree with this function. The host accounts for
its envelope construction and retains its own emission rules.

These functions accept constructed data and existing JSON trees. Native-leaf
projection remains a host responsibility. The mixed-report bridge above supplies native projections and shared container
traversal. Computational queues and identity sets are outside `ConstructedValue` and these APIs. Evaluation keeps
native handles and ordinary values directly; rendering is an explicit final
step. See [library results and rendering](host-integration.md#library-results-and-rendering).

## render_markdown

`render_markdown(value, budget, location)` borrows a `ConstructedValue` and
returns `Result<Vec<u8>, QueryError>`. It uses these layouts:

| Zirql value | Markdown |
| --- | --- |
| Scalar | A paragraph; an integer count appears on its own line |
| Array, tuple, set | One `Value` column, in stored order |
| Record, dictionary | `Key` and `Value` columns, in insertion order |
| Collection of records | Record fields as columns, in input row order |
| Record or dictionary whose values are all records | A leading `Key` column, then record fields |
| Empty collection, record or dictionary | `_No entries._` |
| Nominal member, explicit keyed array | Compact ordinary JSON with its tagged shape |

Record tables use the first-seen union of field labels. A later field adds a
column without reordering earlier columns. Missing fields and absent cells are
blank. A sequence of empty records uses a `Value` column containing `{}` for
each row; a keyed empty record still displays its key.

Strings and field labels retain their content with Markdown escaping. Pipes,
backslashes, backticks, stars, underscores, brackets and tildes are escaped.
Ampersands and angle brackets use HTML entities. CRLF, CR and LF become `<br>`.
Scalar strings also escape `#`, `+`, `-`, `.`, `)` and `=` and encode leading
spaces and tabs as HTML character references. This keeps their content in a
paragraph instead of creating a heading, list or indented code block. Table
cells retain the escaping described above.
Nested cells and structural dictionary keys use compact shared ordinary JSON,
then the same escaping. Nominal members and keyed arrays therefore retain their
explicit tags, axes and present cells. The renderer does not stringify the whole
report before choosing a table layout.

```rust
use zirql::{Budget, ConstructedValue as V, Location, ProgramLimits, render_markdown};

let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 10_000, nesting: 64, stream_items: 100,
});
let result = V::Array(vec![
    V::Record(vec![("name".into(), V::String("Ada".into()))]),
    V::Record(vec![("count".into(), V::Integer(2))]),
]);
let bytes = render_markdown(&result, &mut budget, Location::Query(0..5))?;
assert_eq!(bytes, b"\n| name | count |\n| --- | --- |\n| Ada |  |\n|  | 2 |\n\n");
```

Markdown output includes leading and trailing blank lines, matching the existing
Zirium report layout. The [CSV table example](csv.md#render-a-markdown-table)
selects this format explicitly. The Rust API and CSV executable in this checkout
support it. Zincite report programs and Zirium queries expose an explicit
`markdown` stage through the mixed-report API below. Native leaves require host
projections, and computational queues and identity sets cannot be rendered.

## render_report_markdown

`render_report_markdown(host, value, budget, location)` returns
`Result<Vec<u8>, EvaluationError<H::Error>>`. It uses the same `RenderHost` and
`RenderView` as mixed JSON reports. Constructed nodes keep their data layout,
including when the host selects tagged JSON; native projections are ordinary
JSON scalars, arrays or objects. Nested native subjects become compact JSON in
cells, with the same escaping and ordering as constructed data.

The mixed-report example above also supports Markdown:

```rust
let markdown = zirql::render_report_markdown(
    &mut Host, &report, &mut budget, Location::Query(0..5),
)?;
assert!(String::from_utf8(markdown)?.contains("| count | 3 |"));
```

Zincite's terminal `markdown` emitter leaves completeness and limitations on
`ProgramResult`; the CLI keeps its existing diagnostics and exit status.
Zirium preserves pass-through emission and its native diff change table.
Ordinary diff data uses the shared layouts with the diff CLI's existing framing.
See the [Zincite report recipe](https://zayenz.se/zincite/guides/queries/#markdown-reports)
and [Zirium report recipe](https://zayenz.se/zirium/guides/query-language/#markdown-reports).

## Budget and errors

Pass the same `Budget` used for reading and evaluation. Conversion checks known
container sizes before allocation and charges traversal, produced JSON nodes and
slots, and copied labels and strings. Rendering checks the JSON tree under the
active nesting/reentry guard, then charges actual escaped output bytes before
buffer growth. Copied text and output bytes spend work and result allowance.
Pretty indentation counts too.

Errors preserve the supplied `Location`, including source ranges, and retain
`WorkLimit` or `ResultLimit` through the JSON writer. Failed work stays charged.
Buffered calls return only complete output; the caller receives no partial tree
or bytes on failure. [Runtime limits](runtime-limits.md) explains the request
allowances and diagnostic categories.

Markdown also checks the rectangular cell count, including the header, against
`stream_items` before expanding sparse rows. It charges column storage and each
table cell to work and result allowance, then charges scalar/JSON conversion
and actual escaped output bytes under that same budget. A short sparse input
can exceed the limit when its rows introduce many distinct columns. Limit
failures preserve their supplied location and return no partial Markdown.
