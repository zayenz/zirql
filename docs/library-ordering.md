# Ordering and extrema

Constructed sorting retains collection kind and current order within equal-key
ties. Extrema return an original positional element, choosing the first tie.
Array/selector keys must be present homogeneous integers, finite floats, exact
strings or members of one nominal type. Those contracts do not mix integers and
floats. Missing or incompatible keys fail even after an extremum has a candidate.
See [ordering rules](values.md#sorting-and-optional-extrema).

## sort

`sort(collection, descending=false) -> same collection kind`; `collection | sort`

Sort positional array elements, set elements or dictionary keys.
`sort([3, 1, 3])` returns `[1, 3, 3]`. Empty input retains its kind.
`descending=true` reverses comparisons while preserving ties; the option must
be a literal Boolean. Sets and dictionary keys use structural value order,
including mixed kinds, absence and nested collections. Array elements use the
scalar contract above. Keyed arrays need explicit conversion first.
Native `| sort` uses [host-dependent ordering](library-native.md#native-ordering).

## sort_by

`sort_by(rows_or_dictionary, "field", descending=false)`

`sort_by(dictionary, value, descending=false)`

Sort positional record rows or dictionary values by a literal field, or sort
dictionary values directly with bare `value`. `sort_by({"a": 2, "b": 1}, value)`
returns a dictionary ordered `b`, then `a`. The original values and collection
kind survive. Quoted `"value"` selects a field named value; bare `value` selects
the whole dictionary value. Empty input retains its kind. Missing, absent or
incompatible selector keys fail. `descending` must be a literal Boolean.
Native `| sort_by(query)` instead accepts a host selector; see
[native ordering](library-native.md#native-ordering).

## min and max

`min(positional_array) -> scalar or absence`; `max(positional_array) -> scalar or absence`

Select the smallest or largest original scalar using the key contract above.
`min([3, 1, 1])` returns `1`; `max([])` returns absence. Empty input is ordinary
optional data, so `min([]) default 0` returns `0`. These calls do not accept
sets, dictionaries or keyed arrays. Native `| min` and `| max` retain host-defined
result kinds and empty behavior; see [native ordering](library-native.md#native-ordering).

## min_by and max_by

`min_by(rows, "field") -> record or absence`; `max_by(rows, "field") -> record or absence`

Select an original positional record by a literal field.
`min_by([{"n": 2}, {"n": 1}], "n")` returns `{"n": 1}`.
Empty input gives absence; ties keep the first record. Every key must satisfy
the scalar contract above. These calls have no `descending` option and do not
accept a selector callback. Native selector forms have a separate
[ordering contract](library-native.md#native-ordering).
