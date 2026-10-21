# Collection pipeline stages

The fixed selection operations below accept positional arrays, insertion-ordered
sets and dictionaries, and admitted native selections/maps. They preserve the
collection kind, entry order, native identity and declared empty schemas. Keyed
arrays expose cardinality only; use `entries` or an explicit positional projection
before removing or moving cells. Strings and tuples are refused. Host bindings
may require grouping to select ordinary value syntax. See
[collection pipelines](values.md#constructed-collection-pipelines).

## filter

`items | filter(pattern => condition)`; `items | filter(condition_using_)`

Test each array/set element or dictionary value in current order, retaining the
original values and exact keys. Arrays retain duplicates; sets retain their
established element metadata, including when emptied. `[1, 2, 3] | filter(_ > 1)`
returns `[2, 3]`; `{"a": 1, "b": 3} | filter(_ > 1)` retains `"b": 3`.
Constructed predicates require known Booleans. Native predicates keep their host's
domain subjects and Unknown/completeness policy. Errors fail the request.
Named arguments and `_` denote the same subject; nested predicates rebind `_`
while named outer captures remain available. Preflight checks the whole written
body even for empty input, including pattern and effect admission. Bare
`filter(...)` reads the host's current input.

## head and tail

`items | head(count)`; `items | tail(count)`

Retain the first or last entries in current order. The bound is a nonnegative
integer expression evaluated once before selection. Zero gives an empty
collection of the same kind; oversized bounds retain all entries. Dictionary
keys stay with their values. Runtime arguments use lexical scope and the host's
existing relative-input policy.

## reverse

`items | reverse`

Reverse every entry, including duplicates and equal-key ties. Empty collections
retain their kind and declarations. Reversal preserves required record fields.

## check

`items | check`; `check(operand)`; `check(count, message)`

Pass the unchanged input through on success. Without an operand require nonempty
input; an integer operand requires exactly that nonnegative count, while a string
operand supplies a failure message for the nonempty check. With two operands,
evaluate the count then the string message in written order. `[] | check(0)`
succeeds. A failure reports expected/actual counts and the optional text at the
check location. Known wrong argument types fail before collection iteration,
including for empty input.

## map

`items | map(pattern => expression)`; `items | map(expression_using_)`

Transform each positional element once in order into an eager positional array.
`[2, 3] | map(_ + 1)` returns `[3, 4]`; empty input returns `[]`.
Bodies can construct records/collections or return supported native values.
Bare `map(...)` uses the host's constructed input. Argument scope and strict
pattern rules match filter; preflight still checks empty-input bodies.
Non-positional sources fail. See [map reports](language-reference.md#map-and-flat_map).

## flat_map

`items | flat_map(pattern => sequence)`; `items | flat_map(sequence_using_)`

Evaluate each positional body once and concatenate one level in input order.
`[2, 3] | flat_map([_, _ + 10])` returns `[2, 12, 3, 13]`.
Empty input or empty body sequences contribute no elements. Every body must
produce a positional sequence; sets, tuples, dictionaries, keyed arrays and
opaque native selections fail. Bare `flat_map(...)` uses the constructed input.
Nested argument scope matches map. Expanded output shares the request and stream
limits; failure yields no partial result.

## count

`items | count -> int`; bare `count`

Return array/set element count, dictionary entry count, present keyed-cell count,
or native stream subject count. `[2, 2] | count` gives `2`; empty input gives `0`.
Bare count reads the host's current input. Constructed count uses
[length](library-collections.md#length); native counts can keep their exact host
scalar. A scalar result cannot feed a collection stage.

## unique

`items | unique -> positional sequence or native stream`

Keep the first equal value in incoming order. Positional data may contain mixed
scalars, absence, records, tuples and nested collections. Lists and tuples use
positional identity; sets use membership and dictionaries use associations.
Different constructed kinds stay distinct, and floating signed zeros compare
equal. Declared element contracts remain available on empty results.

Ordinary data exposed through native wrappers uses the same structural identity.
Opaque native leaves use exact host identity, including source owner and diff side.
Two operations or declarations with the same displayed name remain distinct.
Project `names` explicitly to deduplicate spellings. Dictionaries and keyed
arrays require an explicit positional conversion before unique or tally.

## tally

`items | tally -> dictionary of int`

Count the same positional values or native subjects admitted by `unique`.
Keys retain their first actual representatives and first-seen order; counts
use checked signed integers. `["ash", "apple", "ash"] | tally` returns
`{"ash": 2, "apple": 1}`. Empty input returns a dictionary with any available
key declaration and integer value declaration.

Full expressions compose according to that dictionary result. For example,
`["ash", "apple", "ash"] | tally | reverse | head(1)` gives `{"apple": 1}`.
Dictionary filter, count, length, bounds, check and `sort_by(value)` keep their
ordinary contracts. Scalar counts refuse collection continuations. The small
`Pipeline`/`parse_pipeline` API retains its terminal count/tally representation.

String keys render as ordinary ordered objects. Other keys use the shared
[tagged entry encoding](library-rendering.md), with native leaves projected
only for output. Names and rendered descriptors never determine identity.

## Other collection suffixes

`collection | sort` uses [stable same-kind sorting](library-ordering.md#sort).
`items | windows(size)` and `rows | runs_by("field")` use the same contracts as
their [ordinary calls](library-grouping.md). Native selection and ordering use the
same entries, with host admission and restoration of native result shapes. The compact `parse_pipeline` API is a
smaller native reader with literal bounds/messages; richer readers and typed
APIs accept runtime expressions. See
[predicate subjects](native-programs.md#predicate-argument-subjects).
