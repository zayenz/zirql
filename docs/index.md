# Zirql

Zirql is a Rust library for query languages over native data. A host supplies
the data and its domain operations. Zirql evaluates pipelines, programs and
comprehensions, keeping native results available to the caller.

The examples start with a small word host. It splits `birch apple ash apple`
into borrowed words in source order and defines `prefix("a")` as an exact,
case-sensitive prefix test. Repeated words remain separate stream entries.

## Select and count words

```text
filter(prefix("a")) | head(1) | count
```

Filtering keeps `apple`, `ash`, `apple`. Taking the first leaves `apple`, so
the result is `Count(1)`. The words still borrow the source text. The host
implements `prefix`; Zirql supplies filtering, ordering, truncation and count.

## Keep input order

Changing the stage order changes which words reach the filter:

| Pipeline | Returned words |
| --- | --- |
| `filter(prefix("a")) \| head(2)` | `apple`, `ash` |
| `head(2) \| filter(prefix("a"))` | `apple` |

These are native streams. A caller can inspect them directly or ask its host
to render them. Zirql does not choose an output format.

## Construct word records

After a host explicitly projects the selected words to the string array
`["apple", "ash", "apple"]`, a comprehension can construct labelled records.
Written as expression-tree notation:

```text
[{word: w, label: "word:" ++ w} for w in words]
```

The returned data, shown here as JSON, is:

```json
[
  {"word": "apple", "label": "word:apple"},
  {"word": "ash", "label": "word:ash"},
  {"word": "apple", "label": "word:apple"}
]
```

The [worked examples](examples.md) explain the word model and these results.
The common text reader accepts the pipelines above. Comprehensions use the
Rust value representation; a frontend supplies their text syntax.

## Read the documentation

- [Language reference](shared-concepts.md) explains pipelines, predicates,
  programs, values and evaluation limits.
- [Examples](examples.md) follows words through queries to native results and
  constructed data. [Design examples](language-direction.md) use the same
  small domain to explain unimplemented language forms.
- [Embedding in Rust](host-integration.md) explains what an integration needs.
  [Getting started](getting-started.md) has a pinned dependency and a runnable
  scalar expression.

[Semantic model integrations](semantic-models.md) shows how Zincite and Zirium
adapt the shared evaluator to source selections and operation handles. Their
predicates, syntax, identity rules and renderers belong to those hosts.

For Rust signatures, use `cargo doc --open` for your pinned revision.
[docs.rs](https://docs.rs/zirql) covers published versions.

Zirql is developed by [Mikael Zayenz Lagerkvist](https://zayenz.se/).
[Source on GitHub](https://github.com/zayenz/zirql) · MIT OR Apache-2.0
