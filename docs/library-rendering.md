# JSON rendering

Zirql converts constructed results to JSON after evaluation. The caller chooses
the representation and layout, passes the request's remaining budget, and
receives a complete buffer. These Rust APIs belong to the working checkout;
published Zirql 0.2.0 does not include them. They are not query-language calls.

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
projection remains a host responsibility. A shared bridge for mixed native and
constructed containers and adoption in Zincite and Zirium remain separate
integration work. Computational queues and
identity sets are outside `ConstructedValue` and these APIs. Evaluation keeps
native handles and ordinary values directly; rendering is an explicit final
step. See [library results and rendering](host-integration.md#library-results-and-rendering).

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
