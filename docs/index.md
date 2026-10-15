# Zirql

Zirql lets users write their own analyses over the data an application exposes.
Inspired by awk and jq, it uses expressions and small programs to select data
and build reports.

For a list of words, you might want to find neighbouring pairs or retain every
position of a repeated word. The examples below start with this small model. An
application can also expose its own objects, such as source nodes, and let
queries follow their relationships.

## Analyse a list of words

Input: `words = ["birch", "apple", "ash", "apple"]`. Choose an analysis below.
Results are shown as JSON.

<!-- examples:start -->
### Adjacent word pairs

Which words occur next to each other? Walk the positions and join each word
with its neighbour:

```text
[words[i] ++ " " ++ words[i + 1]
 for i in 1..(length(words) - 1)]
```

```json
["birch apple", "apple ash", "ash apple"]
```

Positions start at 1. Each pair overlaps the next, so the result retains the
order of the text. An input with fewer than two words produces no pairs.

### Select words

Keep words beginning with `a`:

```text
[w for w in words if starts_with(w, "a")]
```

```json
["apple", "ash", "apple"]
```

The comprehension keeps input order and duplicates. String matching is exact
and case-sensitive.

### Build a summary

Construct a record with counts and an alphabetical list:

```text
{
  "count": length(words),
  "beginning_with_a": length([w for w in words if starts_with(w, "a")]),
  "alphabetical": sort(words)
}
```

```json
{
  "count": 4,
  "beginning_with_a": 3,
  "alphabetical": ["apple", "apple", "ash", "birch"]
}
```

Record fields can contain expressions and nested collections.
<!-- examples:end -->

[Work through the word analysis](examples.md), or [run these expressions from
Rust](getting-started.md). The examples use the parser and evaluator in the
development checkout; see the setup guide for version requirements.

## Learn the language

The [language reference](language-reference.md) covers expressions, data types,
comprehensions, pipelines and programs. Start with values and collections. The
later chapters explain how queries work with objects supplied by an application.

Zirql has immutable bindings and typed named functions. Evaluation has explicit
work and result limits chosen by the application. Errors carry locations, so the
application can show which expression failed.

## Embed Zirql

For this word model, the application splits text and supplies a string array
named `words`. Zirql parses and evaluates the analysis. An application working
with source code can instead supply native nodes, projections and navigation.
Those nodes keep their identity and connection to the source.

The [implementation reference](implementation-reference.md) explains how to
supply data and connect domain operations to the language. It starts with the
word model, then uses Zincite and Zirium to explain choices such as source
ownership and object identity. Their own sites document application usage.

Zirql is developed by [Mikael Zayenz Lagerkvist](https://zayenz.se/).
[Source on GitHub](https://github.com/zayenz/zirql) · MIT OR Apache-2.0
