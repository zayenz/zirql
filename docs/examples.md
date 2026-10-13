# Query examples

The word host reads `birch apple ash apple` with `split_whitespace`. Its input
is an ordered stream of borrowed string slices:

```text
birch, apple, ash, apple
```

There are four entries, including two occurrences of `apple`. This host defines
`prefix("text")` as a case-sensitive `starts_with` test. Every word has a known
answer, so its predicates return True or False. Zirql combines those answers
and runs the shared stages. No compiler, file-format parser or renderer is
needed for these queries.

## Select and count borrowed words

```text
filter(prefix("a")) | head(1) | count
```

| Step | Result |
| --- | --- |
| Input | `birch`, `apple`, `ash`, `apple` |
| `filter(prefix("a"))` | `apple`, `ash`, `apple` |
| `head(1)` | `apple` |
| `count` | `Count(1)` |

The [complete Rust host example](../README.md#a-borrowed-source-query) supplies
this predicate and its parser callback, admits the words with a budget and
evaluates the pipeline. It returns the native count before any rendering.

## Combine predicates

Keep words beginning with `a`, except those beginning with `ap`:

```text
filter(prefix("a") and not prefix("ap"))
```

The returned stream contains `ash`. Adding `| count` returns `Count(1)`.
Using `or` instead gives a different selection:

```text
filter(prefix("ap") or prefix("b"))
```

The result is `birch`, `apple`, `apple`, in input order. The host evaluates each
prefix test. Zirql handles `and`, `or`, negation and short-circuiting. Hosts
with incomplete facts may also return Unknown; filtering keeps only known
true answers, and negation preserves Unknown.

## Change the stage order

```text
filter(prefix("a")) | head(2)
```

This keeps `apple`, `ash`. Filtering first makes `head` take the first two
matching words. Swapping the stages:

```text
head(2) | filter(prefix("a"))
```

The head is `birch`, `apple`, so filtering it leaves only `apple`. Neither
query sorts the input. Duplicates survive unless an explicit stage removes them.

## Count an empty selection

```text
filter(prefix("z")) | count
```

No word matches, so the result is `Count(0)`. An empty input also returns zero.
`count` is terminal: `count | head(1)` is rejected before evaluation.

## Build records from words

Native streams and constructed arrays have different representations. To build
a scalar report, the word host explicitly projects the filtered slices to
`ConstructedValue::String` values in a positional array. That gives a constructed
binding `words = ["apple", "ash", "apple"]`.

This notation describes a comprehension over that binding:

```text
[{word: w, label: "word:" ++ w} for w in words]
```

Each candidate binds `w` to one string. The head creates a record with `word`
first and `label` second; `++` concatenates strings. The returned data, rendered
here as JSON for readability, is:

```json
[
  {"word": "apple", "label": "word:apple"},
  {"word": "ash", "label": "word:ash"},
  {"word": "apple", "label": "word:apple"}
]
```

The repeated word produces a repeated record. Array order follows generation
order. The core returns constructed values, while the host decides whether
to print JSON, return a Rust value or use another representation.

The comprehension is notation for `ValueExpression`, not syntax accepted by
`parse_pipeline`. See [values and comprehensions](values.md) for the callable
representation and [embedding in Rust](host-integration.md#constructed-value-apis)
for its evaluation and copy boundaries.

## Filter before constructing a value

The host can also project the four word lengths in UTF-8 bytes. These ASCII
words give `sizes = [5, 5, 3, 5]`. In expression-tree notation:

```text
[n * 2 for n in sizes if n > 3]
```

The result is `[10, 10, 10]`. Its positions are 1, 2 and 3, newly assigned by
the positional constructor. They do not retain input positions 1, 2 and 4.
Use explicitly [keyed arrays](values.md#keyed-arrays) when those positions need
to survive as keys.

All four candidates are charged before filtering, including the rejected
size 3. The head runs only for the three survivors.

## Integrate another semantic model

<span id="count-mlir-operations-by-name"></span>
<span id="build-records-from-labels"></span>

[Semantic model integrations](semantic-models.md) applies these language rules
to Zincite and Zirium. Its operation-count and label-report examples use each
host's own inputs and syntax.

The common reader and [native projection APIs](host-integration.md#correlated-native-projection)
support correlated rows and scalar summaries. The core admits
[required typed parameters](native-programs.md#required-caller-parameters)
through immutable bindings. The [design examples](language-direction.md)
show complete candidate surfaces, including proposed matching and paths.

## Project each native subject

A host with a `names` string projection and native `count` stage can read:

```text
project(w, {"name": w | names | one, "n": w | count})
```

For native subjects `beta`, `alpha`, `beta`, `evaluate_projection` returns:

```json
[{"name": "beta", "n": 1}, {"name": "alpha", "n": 1}, {"name": "beta", "n": 1}]
```

The core returns constructed records before rendering. The subject stays native
until `names` projects strings. `one` requires exactly one projected name, and
`count` checks the signed integer boundary when converted to constructed data.
The repeated subject produces a repeated row.

A field such as `require(false, w | failing_stage | names, "stop")` is fully
preflighted, then fails with `stop` without executing `failing_stage`. Undefined
names, known incompatible types and effects still fail preflight.
