# Pipelines

The [word tutorial](examples.md) works with an array that Zirql can inspect
directly. A host can also expose an ordered stream of native items, such as
borrowed words or document nodes. A pipeline applies stages to that stream.

The examples here use the same text, `birch apple ash apple`, through the
[borrowed-word host](../README.md#a-borrowed-source-query). That host defines
the `prefix` predicate. Zirql supplies filtering, stage order and Boolean
composition. These pipelines use a different adapter from the tutorial's
constructed-array executable.

## Filter and take a prefix

```text
filter(prefix("a")) | head(2)
```

Filtering keeps `apple`, `ash`, `apple`; `head(2)` returns `apple`, `ash`.
Swapping the stages changes the answer:

```text
head(2) | filter(prefix("a"))
```

The first two input words are `birch`, `apple`, so this returns only `apple`.
Neither stage sorts or deduplicates. `head` accepts a nonnegative integer;
`head(0)` returns an empty stream.

## Count results

```text
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

```text
filter(prefix("a") and not prefix("ap"))
```

The result is `ash`. `and` stops when false is decisive, and `or` stops when
true is decisive. A host can also return unknown when it lacks a fact.
Negation preserves unknown, and filtering keeps only known true results.
See [diagnostics and limits](diagnostics-and-limits.md).

## Additional stages

The native pipeline representation also supports stable unique, string tally
and host-defined operations. Their text spellings and availability belong to
the frontend.

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

The small `parse_pipeline` API reads `filter`, `head` and terminal `count`,
using a callback for predicate leaves. It does not parse every operation in
the native representation. Broader frontends register domain stages through
the common native readers or lower their own parser output. The
[implementation reference](host-integration.md#parse-and-lower) explains the
entry points.

## Programs and constructed data

[Programs](native-programs.md) save intermediate values and combine native
selections. Explicit projections construct scalar reports, which can use the
same [value expressions and comprehensions](values.md) as the word tutorial.
All nested calls share the active request budget.
