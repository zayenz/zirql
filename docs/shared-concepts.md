# Pipelines

A pipeline applies stages in written order. Each stage receives the preceding
result. Start with the [word example](getting-started.md), whose input is
`words = ["ash", "apple", "birch", "apple"]`:

```sh
cargo run --locked --example words -- 'words | filter(starts_with("a")) | sort | unique'
```

Filtering retains `ash`, `apple`, `apple`. Sorting puts apple before ash, then
`unique` keeps the first of each equal value. The result is `["apple", "ash"]`.
The executable prints values in Rust's `Debug` format.

## Choose the source and stages

An explicit source supplies its own pipeline input:

```zirql
["birch", "ash", "apple"] | filter(starts_with("a")) | count
```

This returns 2. In the word host, bare `count` instead counts its four input
words. Each field in this dictionary starts from that same input:

```zirql
{"selected": filter(starts_with("a")) | count, "total": count}
```

It returns `{"selected": 3, "total": 4}`. Computing `selected` does not replace
the input seen by `total`. A saved value can be a source too. The
[CSV walkthrough](csv.md) saves a selection with `let`, then continues it with
`selected | count`.

Constructed filtering also accepts a scoped condition:
`words | filter(ends_with(_, "h"))` keeps the same words as
`[w for w in words if ends_with(w, "h")]`. `_` denotes the current element;
`filter(w => ends_with(w, "h"))` names it explicitly. The `starts_with("a")`
shorthand shown above checks a case-sensitive prefix.

## Keep order explicit

Arrays preserve input order and duplicates until an operation changes them.
`sort` orders values; `unique` keeps first occurrences. To count spellings while
retaining their first-seen order:

```sh
cargo run --locked --example words -- 'words | tally'
```

The dictionary contains `ash: 1`, `apple: 2`, `birch: 1`, in that order. Sorting
before tally changes that first-seen order. A dictionary also supports explicit
same-kind ordering with `sort(d)` by string key or `sort_by(d, value)` by value.
Equal sort keys keep their incoming order, including with `descending=true`.
See [collection order](values.md#selected-collection-syntax-and-ordering).

A stage requires an appropriate input kind. `count` returns a scalar, so
`words | count | unique` fails instead of treating the count as a collection.
The constructed tally suffix also ends that pipeline. Broader native frontends
can continue dictionaries through sorting and bounds, according to their
supported stages. Their emitters have their own terminal or pass-through rules.

## Select and project native objects

Native pipelines start from objects supplied by the application. Zirium starts
with MLIR operations, so a query can follow SSA users:

```zirql
filter(op("arith.addi")) | users | filter(has_attr("analysis.tag"))
```

Zincite starts with MiniZinc source items and can select their written calls:

```zirql
filter(kind("constraint")) | expressions | call_names | tally
```

The host defines `op`, `users`, `kind`, `expressions` and `call_names`. Zirql
supplies shared sequencing, filtering and aggregation. Projecting names makes
strings available for calculation. Saving a native selection retains the
original objects and their source context until rendering.

Navigation can retain duplicate objects. Stable native `unique` compares the
identity supplied by the host, which may differ from equality of projected
names. Two operations with the same name are still different operations.
Set union keeps first appearance from the left operand, then the right;
intersection and difference deduplicate in left-operand order.

The [Zirium](https://zayenz.se/zirium/guides/query-language/) and
[Zincite](https://zayenz.se/zincite/guides/queries/) guides provide complete
commands, domain stages and output rules. [Programs and native data](native-programs.md)
explains saved native values, projections and correlated selections.

## The small embedding reader

The Rust `parse_pipeline` API is a smaller entry point. It reads `filter`,
`head`, `unique` and terminal `count` or `tally`, using a callback for predicate
leaves. For example, the [borrowed-word host](../README.md#a-borrowed-source-query)
registers `prefix`:

```zirql
filter(prefix("a")) | head(3)
```

It returns ash, apple, apple. Reversing those stages to
`head(3) | filter(prefix("a"))` returns ash, apple because birch occupied the
third input position. Neither stage sorts or deduplicates. `head(0)` returns an
empty stream. `one(a)` has a different purpose: it requires exactly one value
and fails on zero or several.

Predicates support `not`, `and`, `or` and parentheses, in that precedence order.
`and` stops when false is decisive; `or` stops when true is decisive. Hosts can
also supply unknown facts. Negation preserves unknown, and filtering keeps only
known true results. See [diagnostics and limits](diagnostics-and-limits.md).

This API's terminal rules apply to its small representation. Full native
frontends use common native readers or lower their own syntax to the broader
program representation. [Embedding in Rust](host-integration.md#parse-and-lower)
explains those entry points and the retained native result context.
