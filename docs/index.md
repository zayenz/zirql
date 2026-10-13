# Zirql

Zirql is a Rust library for building query languages. It is developed for
[Zirium](https://github.com/zayenz/zirium), which inspects MLIR operations, and
[Zincite](https://zayenz.se/zincite/), which works with MiniZinc source and data.
The library evaluates pipelines and programs. Each tool supplies its domain
operations and keeps ownership of its source objects.

## Filter native items

A host can give the common reader a predicate such as `prefix`. For the borrowed
words `birch apple ash`, this query keeps the first word starting with `a` and
counts it:

```text
filter(prefix("a")) | head(1) | count
```

The native result is `Count(1)`. The input words can keep borrowing their source
text throughout evaluation. The [pipeline guide](shared-concepts.md) explains
the reader and its host callback.

## Use Zirql in a Rust tool

[Getting started](getting-started.md) has a pinned dependency and an expression
you can run.

- [Pipelines](shared-concepts.md) for ordered selections and host predicates.
- [Native programs](native-programs.md) for saved bindings and emission.
- [Values and comprehensions](values.md) for arrays, records and keyed data.
- [Host integration](host-integration.md) for domain hooks and source ownership.
- [Diagnostics and limits](diagnostics-and-limits.md) for located failures and
  request budgets.

The APIs are available and evolving. Use `cargo doc --open` for the revision you
build against; [docs.rs](https://docs.rs/zirql) covers published versions.

## Language under development

The [language examples](language-direction.md) explore reports that keep each
function's name beside its operation counts, compare demand with capacity at
matching keys, and retain native captures from structural matches. Projection,
reducers and typed parameters are part of that design work. These are design
examples; their syntax is not implemented.

For queries you can run in a domain tool, read the
[Zincite guides](https://zayenz.se/zincite/) or
[Zirium documentation](https://github.com/zayenz/zirium).

Zirql is developed by [Mikael Zayenz Lagerkvist](https://zayenz.se/).
[Source on GitHub](https://github.com/zayenz/zirql) · MIT OR Apache-2.0
