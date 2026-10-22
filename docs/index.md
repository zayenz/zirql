# Zirql

These guides cover Zirql 0.3.0. See [setup](getting-started.md) for the Rust
dependency and runnable word and CSV examples.

Zirql lets users write analyses that an application's built-in reports do not
cover. A query can select the items of interest, combine related values and
calculate a result. Inspired by awk and jq, Zirql uses expressions and small
programs to describe that work.

For a list of words, you might want to find neighbouring pairs or retain every
position of a repeated word. The examples below start with this small model. An
application can also expose its own objects, such as source nodes, and let
queries follow their relationships.

## Analyse a list of words

Start with a short piece of text, split into the array
`words = ["ash", "apple", "birch", "apple"]`. We can use Zirql to find neighbouring word
pairs, select words by their spelling, or build a summary of the list.

Each tab below shows one of these analyses. The queries all use the same input,
and their results are shown as JSON.

<!-- examples:start -->
### Adjacent word pairs

Which words occur next to each other? Walk the positions and join each word
with its neighbour:

```zirql
[words[i] ++ " " ++ words[i + 1]
 for i in 1..(length(words) - 1)]
```

```json
["ash apple", "apple birch", "birch apple"]
```

Positions start at 1. Each pair overlaps the next, so the result retains the
order of the text. An input with fewer than two words produces no pairs.

### Select words

Keep words beginning with `a`:

```zirql
[w for w in words if starts_with(w, "a")]
```

```json
["ash", "apple", "apple"]
```

The comprehension keeps input order and duplicates. String matching is exact
and case-sensitive. The same comprehension can use
`[w | w in words where starts_with(w, "a")]`; both spellings follow the
[shared comprehension rules](language-reference.md#comprehensions).

### Distinct words

Sort the selection and keep each spelling once:

```zirql
[w for w in words if starts_with(w, "a")] | sort | unique
```

```json
["apple", "ash"]
```

`unique` retains each first occurrence. Sorting first gives alphabetical output.

### Word frequencies

Count how often each word occurs in the input:

```zirql
(words) | tally
```

```json
{"ash": 1, "apple": 2, "birch": 1}
```

The result has one field per word, with its frequency as the value. Both
occurrences of `apple` contribute to its count. Fields follow the first
occurrence of each word.

### Build a summary

Construct a record with counts and an alphabetical list:

```zirql
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
Rust](getting-started.md).

## Learn the language

Use the [library reference](library-reference.md) to find functions, pipeline
stages and operators by purpose or alphabetical name.

The [language reference](language-reference.md) covers expressions, data types,
comprehensions, pipelines and programs. Start with values and collections. The
later chapters explain how queries work with objects supplied by an application.

An analysis can start as a single expression. When calculations need to be
reused, immutable bindings and typed functions give them names. The same
collection operations apply to literal data and values supplied by an
application.

## Embed Zirql

For this word model, the application splits text and supplies a string array
named `words` through `ConstructedBinding` and `ConstructedProgram`.
The [minimal Rust example](embedding-programs.md#constructed-data-programs) shows
that boundary. Zirql parses and evaluates the analysis. An application working
with source code can instead supply native nodes, projections and navigation.
Those nodes keep their identity and connection to the source. The application
chooses which operations queries may use and sets limits on their work and
results.

The [implementation reference](implementation-reference.md) explains how to
supply data and connect domain operations to the language. It starts with the
word model, then uses Zincite and Zirium to explain choices such as source
ownership and object identity. Their own sites document application usage.

To query files, start with the [CSV examples](csv.md). Then
[build the CSV host](csv-host.md) to see how the Rust application exposes those
tables to Zirql and how to adapt it to your own data.

Zirql is developed by [Mikael Zayenz Lagerkvist](https://zayenz.se/).
[Source on GitHub](https://github.com/zayenz/zirql) · MIT OR Apache-2.0
