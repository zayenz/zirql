# Zirql

Zirql is a Rust library for query languages over native data. It evaluates
pipelines, programs and comprehensions for tools such as
[Zirium](https://github.com/zayenz/zirium) and
[Zincite](https://zayenz.se/zincite/). A host supplies domain operations and
retains ownership of its source objects.

## Count MLIR operations by name

Suppose a Zirium selection contains two `arith.addi` operations and one
`arith.muli`. This query projects their names and counts each name:

```text
names | tally | json
```

```json
{"arith.addi": 2, "arith.muli": 1}
```

Zirium supplies `names` and the JSON emitter. Zirql supplies the string tally.
The [worked operation example](examples.md#count-mlir-operations-by-name)
explains that boundary.

## Select and count words

Given the borrowed words `birch apple ash`, a host that implements `prefix`
can run this pipeline:

```text
filter(prefix("a")) | head(1) | count
```

Filtering keeps `apple` and `ash`. Taking the first leaves `apple`, so the native
result is `Count(1)`. The words still borrow their source text. See the
[worked pipeline example](examples.md#select-and-count-borrowed-words).

## Construct a report

For `labels = ["alpha", "beta"]`, Zincite and Zirium can construct the same
records with their own syntax. Zirium writes:

```text
labels = names;
[{"name": n, "label": "data:" ++ n} for n in labels] | json
```

The constructed value, rendered here as JSON, is:

```json
[
  {"name": "alpha", "label": "data:alpha"},
  {"name": "beta", "label": "data:beta"}
]
```

The [comprehension example](examples.md#build-records-from-labels) explains the
input, ordering and equivalent Zincite spelling. These host frontends use the
shared value evaluator; the core does not parse this report syntax itself.

## Read the documentation

- [Language reference](shared-concepts.md) explains pipelines, predicates,
  programs, values and evaluation limits.
- [Examples](examples.md) follows concrete inputs through queries to results.
  The separate [design examples](language-direction.md) cover unimplemented
  language forms.
- [Embedding in Rust](host-integration.md) explains what an integration needs.
  [Getting started](getting-started.md) has a pinned dependency and a runnable
  scalar expression.

For queries over MLIR or MiniZinc files, read the
[Zirium documentation](https://github.com/zayenz/zirium) or
[Zincite guides](https://zayenz.se/zincite/). For Rust signatures, use
`cargo doc --open` for your pinned revision. [docs.rs](https://docs.rs/zirql)
covers published versions.

Zirql is developed by [Mikael Zayenz Lagerkvist](https://zayenz.se/).
[Source on GitHub](https://github.com/zayenz/zirql) · MIT OR Apache-2.0
