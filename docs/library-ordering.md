# Ordering and extrema

Sorting preserves original values, including native handles, and keeps incoming
order within equal-key ties. Extrema choose the first tie unless an `_all` form
is used. The input and selector forms below determine how keys are obtained.
Native inputs require host support; they use the same operation names.

## sort

`sort(collection, descending=false) -> same collection kind`

`items | sort`

Sort positional array elements, set elements or dictionary keys.
`sort([3, 1, 3])` returns `[1, 3, 3]`. Empty input retains its kind.
`descending=true` reverses comparisons while preserving ties; the option must
be a literal Boolean on the function form. Sets and dictionary keys use
structural value order, including mixed kinds, absence and nested collections.
All selected keys use the same structural kind order: absence, Boolean,
integer, finite float, string, nominal member, array, tuple, set, dictionary,
then keyed array. Integers and floats retain distinct kinds. Native unsigned
integers share the integer rank and compare exactly across the full `u64` range.
Arrays and tuples compare positionally; sets and dictionaries compare canonical
membership and associations. Strings use exact byte order and nominal members
use coherent type identity and declaration order.
Keyed arrays need explicit conversion first.

The pipeline form also accepts native inputs with a host-supplied default key.
The host admits the input kind and key; sorting retains the native values and
their owners. Native dictionaries sort by key. Use `sort_by` when native items
need an explicit key query. See [native key admission](native-programs.md#order-native-selections).

## sort_by

`sort_by(rows_or_dictionary, "field", descending=false)`

`sort_by(dictionary, value, descending=false)`

`selection | sort_by(query)`; `dictionary | sort_by(value)`

Sort positional record rows or dictionary values by a literal field, or sort
dictionary values directly with bare `value`. `sort_by({"a": 2, "b": 1}, value)`
returns a dictionary ordered `b`, then `a`. The original values and collection
kind survive. Quoted `"value"` selects a field named value; bare `value` selects
the whole dictionary value. Empty input retains its kind. All keys use `sort`'s
structural contract, including actual absence, mixed kinds and compound values.
Missing fields, invalid row shapes, invalid selector cardinality and incoherent
nominal metadata fail.
`descending` is a literal Boolean option on the function forms.

For a native selection, the pipeline form runs a pure query against each
singleton subject to obtain one admitted key. For example, if `name` projects
one string, `input | sort_by(name)` orders the original subjects by that string.
Queries run in input order before comparisons; a later key failure still fails
the operation. Native dictionary `sort_by(value)` orders by values without
turning the dictionary into an array of entries. Host key and result-kind
admission remains in force. See [native selectors and dictionaries](native-programs.md#order-native-selections).

## min and max

`min(positional_array) -> element or absence`; `max(positional_array) -> element or absence`

`selection | min`; `selection | max`

Select the smallest or largest original element using `sort`'s structural contract.
`min([3, 1, 1])` returns `1`; `max([])` returns absence. Empty input is ordinary
optional data, so `min([]) default 0` returns `0`. These calls do not accept
sets, dictionaries or keyed arrays.

On native input, the pipeline forms use the host's default key and retain the
selected native value. Empty native extrema return an empty selection retaining
its kind, element declaration and owner after normal selector and effect
preflight. Constructed single extrema return typed absence when a declaration
is available.

## min_by and max_by

`min_by(rows, "field") -> record or absence`; `max_by(rows, "field") -> record or absence`

`selection | min_by(query)`; `selection | max_by(query)`

The function forms select an original positional record by a literal field.
`min_by([{"n": 2}, {"n": 1}], "n")` returns `{"n": 1}`.
Empty input gives absence; ties keep the first record. Every key must satisfy
`sort`'s array key contract. These calls have no `descending` option and accept
a literal field rather than a callback.

The native pipeline forms use the same pure, singleton-relative query selector
as `sort_by(query)`, retaining the original native subject. Ties keep the first
subject; empty input retains the native selection kind described under `min` and `max`.
Every key is evaluated, so a later invalid key still fails the operation.

## All-ties extrema

`selection | min_all`; `selection | max_all`

`selection | min_all_by(query)`; `selection | max_all_by(query)`

Select every native item tied at the smallest or largest key, preserving
incoming order and duplicates. The forms without `_by` use the host default
key; the `_by` forms use a pure singleton-relative query. With keys `2, 1, 1`,
`min_all_by(key_query)` retains the latter two subjects. Key admission and empty
behavior follow the other native extrema. These pipeline forms require native
selection support; constructed array calls use `min` and `max` to select one
value or absence.

See the [value ordering rules](values.md#sorting-and-optional-extrema) and
[native embedding contract](native-programs.md#order-native-selections) for the
corresponding key and host interfaces.
