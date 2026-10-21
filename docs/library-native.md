# Shared native forms

These forms need a supporting host frontend and native adapter. They are shared
Zirql mechanisms, but do not make a domain operation available automatically.
The examples here use illustrative host bindings, predicates and edges, not
the word executable. Native handles retain source ownership until explicit
rendering. Preflight checks native kinds and permitted pure operations even
for empty input. Core expression steps own preparation and composition; adapters supply domain
leaves, identity, shape and admission. See [native programs](native-programs.md)
and [Rust composition builders](embedding-programs.md#build-native-composition-steps); actual domain names,
source edits and output policies belong to the
[Zirium](https://zayenz.se/zirium/guides/query-language/) and
[Zincite](https://zayenz.se/zincite/guides/queries/) guides.

## input

`input -> current native input`

Read the incoming native value in this query scope. For an illustrative host
selection `nodes`, a body such as `input union neighbours` explicitly retains
the current subjects and adds their neighbours. Nested native bodies establish
their documented subject input; lexical bindings remain separate.
Top-level statements each start from the original input. Constructed bare
filter/map/count instead read the host's constructed input.
See [statement scope](native-programs.md#statements-and-scope).

## Collection selection and cardinality

[Filter, head, tail, reverse and check](library-pipelines.md#filter) share their
collection contract with ordinary values. Native predicates retain host domain
subjects and Unknown/completeness behavior; source/diff ownership stays with the
adapter. Dictionary predicates receive values and retained entries keep their
keys. Native streams use their admitted selection/restoration boundary. The small
`parse_pipeline` reader accepts literal bounds/messages; richer readers and typed
APIs accept runtime expressions. See [predicate subjects](native-programs.md#predicate-argument-subjects).

## Native unique and tally

Read the shared [unique](library-pipelines.md#unique) and
[tally](library-pipelines.md#tally) contracts. Native subjects retain exact
owners, source coordinates and diff sides. Use an explicit scalar projection
when the intended key is a name or another domain property.

## Native ordering

The [ordering reference](library-ordering.md) covers value and native inputs
together: [sort](library-ordering.md#sort), [sort_by](library-ordering.md#sort_by),
[min/max](library-ordering.md#min-and-max) and
[min_by/max_by](library-ordering.md#min_by-and-max_by).
Native forms retain their subjects and owners; the host supplies key admission,
result kinds and empty-input policy. The [embedding contract](native-programs.md#order-native-selections)
describes those hooks.

## All-ties extrema

See [all-ties extrema](library-ordering.md#all-ties-extrema) for `min_all`,
`max_all`, `min_all_by` and `max_all_by`.

## project

`selection | project(subject, {"field": expression, ...})`

Construct a report row for each native subject. If `text` projects one string,
`project(w, {"word": one(w | text)})` retains one row per input occurrence.
Fields run in written order against the same bound subject; the binder stays
local. Values can retain native children until an explicit rendering boundary.
Empty input still preflights every field. Missing domain operations or invalid
field expressions fail. See [projection](native-programs.md#construct-ordered-report-rows).

## map_by

`selection | map_by(key_query, value_query) -> ordered native dictionary`

Run independent key and value queries for each singleton input, key first.
If `name` returns one string and `children` returns related subjects,
`map_by(name, children | count)` maps names to child counts. Inputs establish
entry order. Keys must be exactly one string; a duplicate fails before its value
runs. Values keep native kind and owner. Empty input returns the admitted empty
dictionary. Host purity, key/result kind and effect checks apply before execution.
See [correlated dictionaries](native-programs.md#correlated-native-dictionaries).

## Native capture match

`match { from name in query; bind other = query; where predicate; }`

Enumerate correlated native captures. For an illustrative `next` relation:
`match { from node in input; bind following = next(node); where true; }`
retains each node with exactly one required following subject.
`from` enumerates candidates; the rightmost varies fastest. `bind` requires
exactly one subject. Later clauses see earlier names, but a source cannot see
its own new name. Duplicate/shadowed names and `input` capture names fail.
False guards reject; unresolved final truth or invalid required bindings fail.
Empty enumeration does not bypass preflight. The `where` predicate receives
the original input plus separate lexical captures. See [native captures](native-programs.md#retain-native-captures).

## paths

`paths(edge, to=predicate, max_depth=n) -> native witness records`

Find one shortest witness per source/sink pair within a required nonnegative
edge bound. For a host exposing `next_word` and `prefix`,
`paths(next_word, to=prefix("birch"), max_depth=2)` can witness ash to birch
through apple. Each result has `source`, `sink`, `steps`, including both
endpoints. An already-matching source has a zero-edge witness.
Sources keep incoming identity order; sinks follow breadth-first discovery;
neighbour order breaks shortest-path ties. Cycles/repeated handles do not
duplicate pairs. The edge is one registered identifier, not a pipeline.
Unknown final target truth and limits fail; an empty bounded result does not
prove global disconnection. See [witness paths](native-programs.md#explain-native-connections).

## where_exists

`selection | where_exists(query)`

Keep the original subject when a pure relation against its singleton input has
a nonempty native result. If `children` is an admitted relation,
`input | where_exists(children)` keeps subjects with children. Preserve incoming
order and duplicates. Complete empty results reject; unresolved nonemptiness
and nested errors fail, including after earlier matches. Host kind, selector and
effect checks apply even to empty input. See [native existence](native-programs.md#native-existence).

## fixpoint

`selection | fixpoint(query)`

Repeatedly replace the selection with the admitted body's result until ordered
identity equality, including duplicates, is unchanged. A host relation
`input | fixpoint(input union neighbours)` explicitly accumulates neighbours.
Intermediate selections do not accumulate implicitly. The final body execution
establishes stability; a repeated non-fixed selection fails cycle detection.
The result must retain the admitted native kind and owner; partial adjacency
cannot prove convergence. Empty-input behavior follows the admitted body and
does not bypass preflight. Body effects follow the host context: Zincite
requires pure bodies; Zirium entry programs also allow their existing
pass-through emitters, including the final stability iteration. Functions and
constructed contexts retain pure-body checks. See
[fixed points](native-programs.md#native-fixed-points).

## Native set operators

`left union right`; `left intersect right`; `left except right`

Combine compatible native selections by host identity, deduplicating the result.
With identities `left = a, b, a` and `right = b, c`, union gives `a, b, c`,
intersection gives `b`, and difference gives `a`. Union keeps left first
appearances then right-only appearances; intersection/difference use left order.
Empty operands obey those membership rules but still receive validation.
Operators have equal precedence and associate left to right; parentheses group
native pipeline operands. The host must register these keywords and compatible
kinds. They are not constructed set operators in the common value reader.
See [native identity and order](shared-concepts.md#select-and-project-native-objects).
