# Word analysis

Start with the text `birch apple ash apple`. Our application splits it at
whitespace and supplies a binding named `words`:

```json
["birch", "apple", "ash", "apple"]
```

This is the whole data model for the first examples: an ordered array of
strings. The application preserves case and punctuation. It does not stem words
or remove duplicates. Both occurrences of `apple` count towards the analysis.

The array expressions in this tutorial run in the [word
example](getting-started.md). Its Rust output uses `Debug`; this guide writes
the same values as JSON for readability. The final section introduces a separate
native pipeline example.

## Select words

A comprehension visits the array and keeps words whose spelling begins with `a`:

```text
[w for w in words if starts_with(w, "a")]
```

```json
["apple", "ash", "apple"]
```

`w` is local to the comprehension. The condition runs for each word, and the
expression before `for`, called the head, produces each selected result. Here
the head is also `w`, so it returns the word unchanged. `starts_with` is built
into Zirql; the application only needs to supply the array.

Use `contains(w, "p")` to select both occurrences of `apple`, or
`ends_with(w, "sh")` to select `ash`. These operations compare exact strings;
`starts_with("Apple", "a")` is false.

## Construct records

Change the head to build a record for each selected word:

```text
[{"word": w, "label": "word:" ++ w}
 for w in words if starts_with(w, "a")]
```

```json
[
  {"word": "apple", "label": "word:apple"},
  {"word": "ash", "label": "word:ash"},
  {"word": "apple", "label": "word:apple"}
]
```

`++` concatenates strings. Records retain their field order, and the array
retains the order of the selected words.

## Find adjacent word pairs

Use positions when the relation between words matters. This expression forms
pairs of neighbouring words, also called bigrams:

```text
[words[i] ++ " " ++ words[i + 1]
 for i in 1..(length(words) - 1)]
```

```json
["birch apple", "apple ash", "ash apple"]
```

Positions are one-based, and `..` includes both endpoints. Stopping one position
before the end leaves room for `words[i + 1]`. A reversed range is empty, so
empty input and a single word both produce an empty array.

## Keep the occurrences in each group

To find every position of a repeated word, group the occurrences by spelling.
First build a record for each occurrence, keeping its position in the array:

```text
group_by(
  [{"word": words[i], "position": i} for i in 1..length(words)],
  "word"
)
```

```json
[
  {"key": "birch", "items": [{"word": "birch", "position": 1}]},
  {"key": "apple", "items": [
    {"word": "apple", "position": 2},
    {"word": "apple", "position": 4}
  ]},
  {"key": "ash", "items": [{"word": "ash", "position": 3}]}
]
```

`group_by` collects records with the same `word` field. Each group has a `key`
and an `items` array. Groups follow the first occurrence of each key, and their
items keep input order. The `apple` group therefore retains positions 2 and 4.

<span id="count-word-frequencies"></span>

Use those groups to report a count and the positions together:

```text
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
`birch` and `ash`. Sorting is stable, so equal counts retain first-seen order.
Sort by `"word"` instead for alphabetical output.

Keeping the positions is the reason to group complete records here. For a
frequency count alone, a host that exposes native string pipelines can use
Zirql's [tally stage](shared-concepts.md#additional-stages). That stage belongs
to the native pipeline interface; it is not a value function in this example.

## Summarise a selection

Count all words, count a selection and sort the original array in one record:

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

`length` counts array entries. It does not count characters in a string. The
selection is an eager array, so Zirql constructs it before counting it.

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

```text
filter(prefix("a")) | head(1) | count
```

Here the host, the Rust application embedding Zirql, implements `prefix` on
borrowed strings. Zirql filters the stream, takes its first matching entry and
returns a count of 1. Stage order matters: filtering before `head(2)` gives
`apple`, `ash`; taking the head first and then filtering leaves only `apple`.

The [setup guide](getting-started.md) shows how to supply the string array used
throughout this tutorial. For richer models, the [implementation
reference](implementation-reference.md) explains constructed values and native
handles. The [adapter examples](semantic-models.md) apply that choice to
source-analysis tools.
