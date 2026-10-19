# Word analysis

Start with the text `ash apple birch apple`. Our application splits it at
whitespace and supplies the array as current input and as the binding `words`:

```json
["ash", "apple", "birch", "apple"]
```

This is the whole data model for the first examples: an ordered array of
strings. The application preserves case and punctuation. It does not stem words
or remove duplicates. Both occurrences of `apple` count towards the analysis.

The array expressions in this tutorial run in the [word
example](getting-started.md). Its Rust output uses `Debug`; this guide writes
the same values as JSON for readability. The final section introduces a separate
native pipeline example.

## Select words

Filter the current input by a literal prefix:

```zirql
filter(starts_with("a"))
```

This returns `["ash", "apple", "apple"]`. Filtering preserves order and repeated
entries. Prefix matching is case-sensitive; an empty prefix matches every
string. This form accepts a literal string prefix. It does not create a
function value.

A comprehension can also select words explicitly:

```zirql
[w for w in words if starts_with(w, "a")]
```

```json
["ash", "apple", "apple"]
```

`w` is local to the comprehension. The condition runs for each word, and the
expression before `for`, called the head, produces each selected result. Here
the head is also `w`, so it returns the word unchanged. `starts_with` is built
into Zirql; the application only needs to supply the array.

Use `contains(w, "p")` to select both occurrences of `apple`, or
`ends_with(w, "sh")` to select `ash`. These operations compare exact strings;
`starts_with("Apple", "a")` is false.

## Sort and retain distinct words

Pipe the completed selection through ascending sorting and stable deduplication:

```zirql
[w for w in words if starts_with(w, "a")] | sort | unique
```

```json
["apple", "ash"]
```

`sort` orders the strings lexically. `unique` keeps each first occurrence. On
its own, `unique` preserves input order, so the selection followed by `| unique`
returns `["ash", "apple"]`.

## Count spellings

Use `tally` to count each selected spelling:

```zirql
[w for w in words if starts_with(w, "a")] | tally
```

```json
{"ash": 1, "apple": 2}
```

The record fields follow each spelling's first occurrence. Counts are checked signed
integers. Tally ends a constructed pipeline; a later stage is refused. Empty
string input gives `[]` for sort or unique and `{}` for tally.

These operations are part of Zirql's common language. The word host supplies
the array and its type facts; it adds no collection parser or evaluator.

## Count unordered neighbouring word pairs

This example requires the development checkout; published `0.1.1` does not
support string sets. From that checkout, run:

```sh
cargo run --locked --example words -- '[{words[i], words[i + 1]} | i in index_set(words) where i < length(words)] | tally'
```

The word host prints this exact Rust `Debug` result:

```text
Array([Record([("key", Set { element_type: Some(String), elements: [String("ash"), String("apple")] }), ("count", Integer(1))]), Record([("key", Set { element_type: Some(String), elements: [String("apple"), String("birch")] }), ("count", Integer(2))])])
```

The first pair occurs once. The next two pairs have the same members in opposite
orders, so tally counts them together. Each `key` remains a typed string set.
The records follow first occurrence, and keys preserve their first insertion
order. Equal adjacent words form a singleton set. An empty or singleton typed
word array yields no pairs and returns `[]`. An ambiguous `[] | tally` still
returns `{}`. See [the value guide](values.md) for the type and host serialization
contracts.

## Construct records

Change the head to build a record for each selected word:

```zirql
[{"word": w, "label": "word:" ++ w}
 for w in words if starts_with(w, "a")]
```

```json
[
  {"word": "ash", "label": "word:ash"},
  {"word": "apple", "label": "word:apple"},
  {"word": "apple", "label": "word:apple"}
]
```

`++` concatenates strings. Records retain their field order, and the array
retains the order of the selected words.

## Find adjacent word pairs

Use positions when the relation between words matters. This expression forms
pairs of neighbouring words, also called bigrams:

```zirql
[words[i] ++ " " ++ words[i + 1]
 for i in 1..(length(words) - 1)]
```

```json
["ash apple", "apple birch", "birch apple"]
```

Positions are one-based, and `..` includes both endpoints. Stopping one position
before the end leaves room for `words[i + 1]`. A reversed range is empty, so
empty input and a single word both produce an empty array.

## Keep the occurrences in each group

To find every position of a repeated word, group the occurrences by spelling.
First build a record for each occurrence, keeping its position in the array:

```zirql
group_by(
  [{"word": words[i], "position": i} for i in 1..length(words)],
  "word"
)
```

```json
[
  {"key": "ash", "items": [{"word": "ash", "position": 1}]},
  {"key": "apple", "items": [
    {"word": "apple", "position": 2},
    {"word": "apple", "position": 4}
  ]},
  {"key": "birch", "items": [{"word": "birch", "position": 3}]}
]
```

`group_by` collects records with the same `word` field. Each group has a `key`
and an `items` array. Groups follow the first occurrence of each key, and their
items keep input order. The `apple` group therefore retains positions 2 and 4.

<span id="count-word-frequencies"></span>

Use those groups to report a count and the positions together:

```zirql
sort_by(
  [{"word": g.key, "count": length(g.items),
    "positions": [item.position for item in g.items]}
   for g in group_by(
     [{"word": words[i], "position": i} for i in 1..length(words)],
     "word")],
  "count", descending=true
)
```

The result starts with `apple`, count `2`, positions `[2, 4]`, followed by
`ash` and `birch`. Sorting is stable, so equal counts retain first-seen order.
Sort by `"word"` instead for alphabetical output.

Keeping the positions is the reason to group complete records here. Use the
constructed `tally` stage above when only the frequency count is needed.

## Summarise a selection

Count all words, count a selection and sort the original array in one record:

```zirql
{
  "count": count,
  "beginning_with_a": filter(starts_with("a")) | count,
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

`count` counts array entries. Each field starts from the same immutable input,
so filtering one field leaves all four words available to another. Filtering is
eager: Zirql constructs the selection before counting it.

An explicit source takes precedence for its own stages:

```zirql
{"distinct": ["apple", "ash", "birch", "apple"] | filter(starts_with("a")) | sort | unique | count, "total": count}
```

This returns `{"distinct": 2, "total": 4}`. Empty input or a filter with no
matches gives count `0`. A missing current input, a scalar or keyed filter
source, or non-string filter elements produce located errors, including an
empty array declared with an incompatible element type. The existing
`starts_with(word, prefix)` helper remains available in scalar expressions.

An empty `words` array gives zero counts and an empty alphabetical list. The
grouped report is also empty. No fallback is needed for these cases.

## Add source identity when it is needed

The positions above identify words in the array. They do not identify byte
ranges in the original text, since splitting discarded the whitespace. To
highlight a word in that text, the application must retain its source location.
It can expose records with text and offsets, or native handles that refer back
to the original source. A native handle identifies an application object; the
application defines its properties and operations.

The [borrowed-word pipeline](../README.md#a-borrowed-source-query) shows a small
native interface over string slices:

```zirql
filter(prefix("a")) | head(1) | count
```

Here the host, the Rust application embedding Zirql, implements `prefix` on
borrowed strings. Zirql filters the stream, takes its first matching entry and
returns a count of 1. Stage order matters: filtering before `head(3)` gives
`ash`, `apple`, `apple`; taking the head first and then filtering leaves
`ash`, `apple`.

The [setup guide](getting-started.md) shows how to supply the string array used
throughout this tutorial. For richer models, the [implementation
reference](implementation-reference.md) explains constructed values and native
handles. The [adapter examples](semantic-models.md) apply that choice to
source-analysis tools.
