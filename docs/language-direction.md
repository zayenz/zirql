# Extending the word model

The [word-analysis example](examples.md) can find neighbouring pairs and group
occurrences using array positions. Highlighting a word in the original document
also needs its source location. This page follows that requirement into a richer
data model. These are possible adapter choices; the runnable word example does
not implement the extensions.

## Retain occurrences

To identify a word in the original text, retain its text and source range. The
host can expose those as record fields or supply a native handle into the
source. With native handles, two occurrences of `apple` are distinct even though
their spellings are equal.

Choose the identity rule before exposing `unique`. Deduplicating occurrences
and deduplicating spellings answer different questions. An explicit text
projection gives the query author control over that choice.

## Add properties

A native word host can supply `text` and `byte_length` projections. The shared
projection construct can then build a record for each subject:

```text
project(w, {"word": one(w | text), "bytes": one(w | byte_length)})
```

The host must register both stages and their result types. Each returns one
scalar per word; `one` checks that cardinality. UTF-8 byte length is a specific
choice. Character count, grapheme count and tokenization policy need their own
definitions if the application exposes them.

## Connect neighbouring words

An occurrence model can expose a `next_word` edge and source positions. Native
matching can retain a word and its successor in the same capture, while a
predicate compares their spellings. This supports questions such as finding
adjacent repeated words without losing either source location.

Zirql checks the scope of capture names and enumerates candidates. The adapter
defines which occurrences are adjacent and how their spellings compare. A
bounded path over `next_word` can also retain intermediate occurrences; the host
should expose it only if a concrete query needs that path.

## Use the same boundaries for another domain

A syntax tree replaces word occurrences with source nodes. An operation graph
replaces adjacency with domain edges. In each case, native values retain their
owner and explicit projections construct report data.

The [adapter examples](semantic-models.md) explain those decisions in Zincite
and Zirium. Their own guides describe application usage. The [host integration
reference](host-integration.md) maps these adapter choices to Rust traits.
