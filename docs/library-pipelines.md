# Constructed pipeline stages

These suffixes apply to constructed sources in the common reader. A host must
identify its constructed bindings; grouping can make an expression source
explicit. Pipelines run left to right after ordinary expression operators.
See [collection pipelines](values.md#constructed-collection-pipelines).
Native stages with similar names follow [native contracts](library-native.md).

## filter

`items | filter(pattern => condition)`; `items | filter(condition_using_)`

Retain positional elements with a known true Boolean result, preserving order
and duplicates. `[1, 2, 3] | filter(_ > 1)` returns `[2, 3]`.
Empty or no-match input gives `[]`. Pattern mismatches, unknown truth and
non-Booleans fail. Bare `filter(...)` uses the host's constructed input.
Named arguments and `_` denote the same subject; nested predicates rebind `_`
while named outer captures remain available. Preflight checks the whole written
body even for empty input. There is no implicit iteration of sets, dictionaries,
keyed arrays or opaque native selections.

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

Count array entries using the constructed `length` reduction. `[2, 2] | count`
returns `2`; empty input returns `0`. Bare count reads the host's constructed
input. The scalar result cannot feed a collection stage. Native count instead
counts the host's incoming stream. See [length](library-collections.md#length).

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
