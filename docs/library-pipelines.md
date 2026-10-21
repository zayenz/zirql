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

`items | unique -> positional array`

Keep each first occurrence in a homogeneous array of Booleans, integers, finite
floats, strings or members of one nominal type. `[3, 1, 3] | unique` gives
`[3, 1]`. Empty input gives `[]`; floating signed zeros compare equal.
Absence, mixed kinds, records, tuples, nested collections and native handles
are unsupported constructed elements. Native unique uses host identity instead.

## tally

`items | tally -> dictionary of int`

Count positional constructed values by structural key identity. Counts use
checked signed integers. `["ash", "apple", "ash"] | tally` returns
`{"ash": 2, "apple": 1}`. Keys and their first representations follow first-seen
order; sets compare by membership and dictionaries by associations. Mixed scalars,
absence and nested collections can be keys. Empty input gives an empty dictionary;
ambiguous `[]` keeps `{}`. General non-string keys need a host-chosen output
encoding. Tally ends a constructed suffix pipeline; no further stage may follow.
Native tally uses the host's projected key and result rules.

## Other collection suffixes

`collection | sort` uses [stable same-kind sorting](library-ordering.md#sort).
`items | windows(size)` and `rows | runs_by("field")` use the same contracts as
their [ordinary calls](library-grouping.md). Those are the remaining constructed
suffixes. Native bounds/check/reversal and native ordering forms require host
support; they are not additional constructed suffixes.
