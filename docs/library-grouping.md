# Grouping and neighbourhoods

These constructed operations consume positional arrays and return eager
positional arrays. They preserve input records and duplicates. Literal field
selectors are not arbitrary callbacks. Keyed arrays need an explicit positional
conversion; native selections need a host-exposed view. Native `unique` and
`tally` use the [shared collection steps](library-pipelines.md#unique)
with host identity and key projections. Their algorithms remain in Zirql.

## group_by

`group_by(rows, "field") -> array of {key, items}`

Group record rows by a present homogeneous integer, string or one nominal type.
`group_by([{"k": "a"}, {"k": "b"}, {"k": "a"}], "k")` returns groups
`a`, then `b`, with two and one rows respectively. Groups follow first-seen
key order; members retain input order. Empty input gives `[]`. Missing fields,
absence and incompatible key kinds fail. This is not consecutive grouping;
use `runs_by` for that. See [stable groups](values.md#stable-record-groups).

Rows can retain native subjects in other fields. For example,
`group_by([{"key": "b", "subject": n1}, {"key": "a", "subject": n2}, {"key": "b", "subject": n3}], "key")`
returns groups `b`, then `a`. The first group's items retain subjects `n1`, then
`n3`, with their original native owners and source coordinates. Sorting retains
declared empty row schemas. These schemas also determine the schema of an empty
grouping result.

## runs_by

`runs_by(rows, "field") -> array of {key, items}`; `rows | runs_by("field")`

Group consecutive rows with structurally equal field values. Keys can include
constructed collections; set/dictionary key order does not affect identity.
`runs_by([{"k": 1}, {"k": 2}, {"k": 1}], "k")` gives three runs, in that
order. A later equal key starts a new run after an intervening different key.
Empty input gives `[]`; missing fields and non-record rows fail.
See [runs and windows](values.md#complete-windows-and-consecutive-runs).

## windows

`windows(items, size) -> array of arrays`; `items | windows(size)`

Return every complete overlapping window with stride one in source order.
`windows([1, 2, 3], 2)` returns `[[1, 2], [2, 3]]`. `size` must evaluate to a
positive integer. Short or empty input gives `[]`; zero, negative and non-integer
sizes fail. There is no padding or stride option. Expansion spends the request
budget and never returns a truncated result on failure.
See [runs and windows](values.md#complete-windows-and-consecutive-runs).

## align

`align(left, right, by="field") -> array of {key, before, after}`

Fully align two record sequences by unique present structural keys on each side.
`align([{"id": "a"}], [{"id": "b"}], by="id")` gives an `a` row with absent
`after`, then a `b` row with absent `before`. Matching rows keep both original
records and the left key's representation. Results follow left order, then
right-only order. Keys can be constructed collections, with order-independent
set/dictionary identity. Both operands run eagerly once, left then right.
Two empty operands give `[]`; one empty side gives unmatched rows. Missing fields,
absent keys, duplicate keys and incompatible sources/rows fail. The call has no
pipeline suffix. Guard an unmatched side before selecting its fields.
See [full alignment](values.md#align-two-record-sequences).
