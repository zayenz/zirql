# Collections and reductions

Constructed arrays, tuples, sets, dictionaries and keyed arrays have distinct
contracts. Collection construction is eager. See [values and comprehensions](values.md)
for worked reports and declared types.

## Arrays and tuples

`[value, ...] -> positional array`; `(value, ...) -> tuple`

`[3, 1, 3]` preserves all three values in order; `[]` is empty. Arrays may mix
constructed types. Tuples have fixed positions: `(3,)` has one element, `()` has
none, and `(3)` only groups an expression. There are no ordinary `array(...)`
or `tuple(...)` constructors in the common reader. Indexed brackets and matrix
forms are described under [keyed arrays](#keyed-arrays).

## Dictionaries and records

`{key: value, ...} -> dictionary`; `(field: value, ...) -> record`

`{"word": "ash", "count": 1}` retains that field order. `{}` is an empty
dictionary. A dictionary key may be any constructed value, including nested
collections; keys are not converted to text. Each key runs before its value;
a duplicate key fails before that value runs. All-string dictionaries use the
record representation and support structural field schemas. A computed key such
as `{name: 1}` reads the binding `name`; quote a static brace field.
`dict<K, V>` and `record {field: T}` are declaration types, not calls.
See [dictionaries](values.md#dictionaries-and-collection-order).

## Sets

`{value, ...} -> set`; `set<T>() -> empty set`

`{3, 1, 3}` retains `3, 1`. Nested and mixed constructed values are allowed;
structurally equal members keep their first occurrence. Equality ignores member
order, but iteration preserves it. Empty `{}` means a dictionary, so use
`set<value>()`, `set<int>()`, `set<bool>()`, `set<float>()` or `set<string>()`
for an empty set. The common reader accepts only those empty-set type arguments;
hosts can supply typed nominal sets through Rust. Strict scalar contracts keep
integer and float elements distinct. See [sets](values.md#nominal-members-and-sets).

## Comprehensions

`[head for pattern in source if condition]`

`[head | pattern in source where condition]`

The filter is optional. The corresponding `{head ...}`, `{key: value ...}` and
`[key: value ...]` forms construct sets, dictionaries and keyed arrays.
`[x * 2 for x in [1, 2] if x > 1]` returns `[4]`. Repeated generators traverse
in written order, rightmost fastest. Later sources see earlier bindings; filters
see the current binding, and heads see every binder. Sources include supported
arrays, sets and integer ranges; native traversal needs a host-exposed view.
Ordinary pattern mismatches fail. `case pattern` skips mismatches before the
filter or head, while errors and unknown conditions still fail. MiniZinc spelling
also permits grouped binders and `name = value` clauses, without `case` on those
single-value clauses. Empty generation retains its established collection kind;
keyed emptiness still needs known domains. Reducers receive the completed
collection. See [scope and generation](values.md#positional-comprehensions) and
[case selection](values.md#select-matching-values).

## Keyed arrays

`[key: value, ...] axes(domain, ...) -> keyed array`

Keys are integers, nominal members or dimension tuples. The optional `axes(...)`
suffix declares complete ordered domains, including missing cells.
`[2: 20, 5: 50] axes([5, 2, 9])` stores cells `5`, then `2`, with key `9` missing.
Cells use row-major domain order. Without axes, present keys establish domains;
an empty keyed result must still establish its typed domains. `[]` alone is
positional, while `[] axes(set<int>(), [8, 1])` retains two axes.
`[3: 7, 8]` assigns successive integer keys `3`, `4`; `[| 1, 2 | 3, 4 |]`
is a rank-two matrix. Axis sources run once left to right in surrounding scope
before entries or generators. Duplicate axis/cell keys, wrong ranks and keys
outside declared axes fail. `keyed<T>` is a declaration type, not a call.
See [keyed construction](values.md#keyed-arrays) for matrix headers and shape rules.

## length

`length(array) -> int`

Count present cells in a positional or keyed array. `length([4, 4])` is `2`;
empty input gives `0`. Missing keyed cells do not contribute. Strings, tuples,
sets and dictionaries are not array operands. Native `count` is a separate
pipeline form. See [array summaries](values.md#array-summaries-and-requirements).

## sum

`sum(array<int>) -> int`

Add every present positional or keyed cell in stored order using checked signed
integer arithmetic. `sum([2, 3])` is `5`; empty input gives `0`. Floats, absence,
non-integer cells and overflow fail. The array is fully constructed first.
See [array summaries](values.md#array-summaries-and-requirements).

## forall

`forall(array<bool>) -> bool`

Return whether every present cell is true. `forall([true, false])` is `false`;
empty input gives `true`. Accept positional or keyed arrays of known Booleans.
The reducer inspects an eager array; a decisive false does not bypass later
construction or errors. Missing keyed cells do not contribute.

## exists

`exists(array<bool>) -> bool`

Return whether any present cell is true. `exists([false, true])` is `true`;
empty input gives `false`. Positional and keyed arrays require known Booleans.
This eager reduction does not skip later construction or errors after a true.
Native `where_exists(query)` instead filters subjects by a native relation.

## one

`one(positional_array) -> element`; native `query | one` with host projection support

Require exactly one element: `one(["ash"])` is `"ash"`. Empty or multiple
elements fail. The result retains the element's kind; it is not a first-element
selection. A native result must expose the required positional view, such as a
projected string list, before extraction. See [native extraction](values.md#native-operands-and-extraction).

## index_set

`index_set(array) -> typed set of keys`

Accept a positional or rank-one keyed array. `index_set([4, 9])` gives `{1, 2}`;
empty positional input gives an empty integer set. A keyed result includes all
declared axis keys, including keys without cells, in axis order, retaining integer
or nominal identity even when empty. Other ranks and kinds fail.
See [keyed keys](values.md#keyed-arrays).

## entries

`entries(keyed_array) -> positional array of {key, value}`

Expose present cells in stored order. `entries([2: 20])` gives
`[{"key": 2, "value": 20}]`. Rank-one keys are scalars; higher-rank keys are
dimension tuples. Empty input gives `[]`. This helper does not accept an ordinary
dictionary or fill missing cells. See [keyed entries](values.md#keyed-entries-and-exact-pairing).

## zip_exact

`zip_exact(left_keyed, right_keyed) -> keyed array of tuples`

Require identical ordered axes and present full keys, including sparse coverage.
`zip_exact([2: 3], [2: 4])` has cell `2: (3, 4)`. Retain the left axes and keys;
equal empty inputs retain their typed shape. Equal cell counts alone do not
establish alignment. Different domains, domain order or missing-cell coverage
fail. No filling, truncation or reindexing occurs.
See [exact pairing](values.md#keyed-entries-and-exact-pairing).
