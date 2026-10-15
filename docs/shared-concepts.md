# Pipelines

The [word tutorial](examples.md) works with an array that Zirql can inspect
directly. A host can also expose an ordered stream of native items, such as
borrowed words or document nodes. A pipeline applies stages in written order.
Constructed arrays accept `| filter(starts_with("prefix"))`, `| count`,
`| sort`, `| unique` and terminal `| tally`. Bare constructed filter/count read
immutable current input supplied by the host. Each sibling record field sees
the same input; explicit sources supply their own stages. See the examples
in the [word tutorial](examples.md#sort-and-retain-distinct-words).

The examples here use the same text, `ash apple birch apple`, through the
[borrowed-word host](../README.md#a-borrowed-source-query). That host defines
the `prefix` predicate. Zirql supplies filtering, stage order and Boolean
composition. These pipelines use a different adapter from the tutorial's
constructed-array executable.

## Filter and take a prefix

```zirql
filter(prefix("a")) | head(3)
```

Filtering keeps `ash`, `apple`, `apple`; `head(3)` retains all three.
Swapping the stages changes the answer:

```zirql
head(3) | filter(prefix("a"))
```

The first three input words are `ash`, `apple`, `birch`, so this returns
`ash`, `apple`.
Neither stage sorts or deduplicates. `head` accepts a nonnegative integer;
`head(0)` returns an empty stream.

## Count results

```zirql
filter(prefix("a")) | count
```

This returns 3. An empty stream counts as zero. `count` is terminal: a later
stage is invalid and fails before evaluation. For instance, `count | head(1)`
cannot take the head of a count.

`head(1)` means "take the first". The value helper `one(a)` instead requires
exactly one element and fails on empty or multiple results.

## Combine predicates

Predicates support `not`, `and`, `or` and parentheses. Precedence is `not`, then
`and`, then `or`:

```zirql
filter(prefix("a") and not prefix("ap"))
```

The result is `ash`. `and` stops when false is decisive, and `or` stops when
true is decisive. A host can also return unknown when it lacks a fact.
Negation preserves unknown, and filtering keeps only known true results.
See [diagnostics and limits](diagnostics-and-limits.md).

## Additional stages

The small pipeline reader also accepts bare `unique` and `tally`. Hosts opt
into their applicability and supply native identity and tally keys. Broader
frontends can expose these same stages together with domain operations.

Stable unique keeps the first item for each native identity key, preserving
input order. String tally counts the keys projected by the host and returns
counts in sorted key order. For word spellings it can directly produce `apple:
2`, `ash: 1`, `birch: 1`. When a host exposes tally, a frequency count needs no
manual grouping. Tally, like count, is terminal.

Identity is a host choice. Two occurrences of the same spelling can be distinct
native objects. Projecting their spellings before comparing them answers a
different question from comparing their source identities.

## Results retain their input context

A pipeline returns a native stream, count or tally with its retained context.
A word can still borrow the source text; an operation handle can still refer to
its document. Rendering happens at an explicit host boundary.

## The common text reader

The small `parse_pipeline` API reads `filter`, `head`, `unique` and terminal
`count` or `tally`, using a callback for predicate leaves. It does not parse every
operation in the native representation. Broader frontends register domain stages through
the common native readers or lower their own parser output. The
[implementation reference](host-integration.md#parse-and-lower) explains the
entry points.

## Programs and constructed data

[Programs](native-programs.md) save intermediate values and combine native
selections. Explicit projections construct scalar reports, which can use the
same [value expressions and comprehensions](values.md) as the word tutorial.
All nested calls share the active request budget.
