# Design examples

The current [word examples](examples.md) select words and construct values.
These sketches use that small domain to combine projection, reducers and
parameters, native captures, grouping and bounded paths.

These complete word-host programs remain design sketches. The core supplies
callable projection, reducers, requirements and typed parameter admission;
common native matching, grouping, keyed helper and bounded path APIs are also
available on the development branch. The word-domain frontends below remain
sketches. A host frontend supplies
its domain syntax and supported subset. Callable behavior is covered by the
[Rust guides](getting-started.md).

## Correlate fields with one native word

Start with `birch apple ash apple`. For this sketch, the word host supplies two
explicit projections: `text` returns a word's string, and `byte_length` returns
its UTF-8 byte count. Each produces exactly one scalar for one word.

```text
param prefix_text: string;
param minimum_bytes: int;

let minimum = require(minimum_bytes >= 0, minimum_bytes,
                      "minimum_bytes must be nonnegative");
let selected = input | filter(prefix(prefix_text));
let rows = selected | project(w, {
  "word": w | text | one,
  "bytes": w | byte_length | one
});
let long_words = [r for r in rows if r.bytes >= minimum];

report {
  "words": long_words,
  "count": length(long_words),
  "total_bytes": sum([r.bytes for r in long_words])
};
```

With `prefix_text = "a"` and `minimum_bytes = 4`, the intended result is:

```json
{
  "words": [
    {"word": "apple", "bytes": 5},
    {"word": "apple", "bytes": 5}
  ],
  "count": 2,
  "total_bytes": 10
}
```

`project(w, {fields})` would bind each native word to `w`, evaluate the fields
in written order and produce one record for that word. The subject binding
would be local to the projection. Both occurrences of `apple` remain present.
`one` would require exactly one value and fail for an empty or ambiguous result.

Zirql would own projection, field correlation and cardinality. The host would
own `prefix`, `text` and `byte_length`. An application could accept a short
query or load a program file; embedding does not require a Zirql executable.

## Required parameters and reductions

The shared `param` reader accepts required `int`, `string` or `bool`
declarations. Library callers supply typed values to `admit_parameters`; a host
CLI can transport repeated `--param name=value` arguments.
Missing, duplicate, extra or incompatible inputs
fail before domain execution, without query-text substitution. Defaults
and named result schemas are outside this design.
The [parameter reference](native-programs.md#required-caller-parameters) defines
decoding and scope; [Rust admission](embedding-programs.md#supply-typed-caller-inputs)
shows the callable entry point.

The candidate reducers `sum`, `length`, `forall` and `exists` operate on evaluated
arrays. Integer sums are checked; quantifiers require known Booleans. These are
eager arrays, so candidate visits and construction costs remain visible in the
request budget.

`require(condition, value, message)` would preflight all operands and evaluate
the condition first. A true condition evaluates the value. A false condition
fails at the requirement with its literal string message. The unselected value
is not evaluated.

## Compare values at matching positions

Suppose the word host projects byte lengths `sizes = [5, 5, 3, 5]`. A caller
supplies per-position capacities `[5, 5, 2, 6]`. Both totals are 18, but position
3 is too small for `ash`. A proposed report could distinguish those questions:

```text
let aligned = require(
  index_set(sizes) == index_set(capacity), capacity,
  "sizes and capacity must have the same index set"
);
let rows = [
  {position: i, bytes: sizes[i], capacity: aligned[i]}
  for i in index_set(sizes)
];
report {
  "totals_fit": sum(sizes) <= sum(aligned),
  "positions_fit": forall([r.bytes <= r.capacity for r in rows]),
  "violations": [r for r in rows if r.bytes > r.capacity]
};
```

The intended report has `totals_fit = true`, `positions_fit = false` and one
violation at position 3. Insufficient capacity is report data. A failed
alignment requirement or arithmetic overflow is an execution error.

This is proposed expression notation. A host frontend chooses its spelling and
renderer; no particular source-file format is implied by these arrays.

## Group rows and align keyed cells

Applied to all four word records, `group_by(rows, "bytes")` produces a
group keyed by integer 5 containing `birch`, `apple`, `apple`, followed by a
group keyed by integer 3 containing `ash`. Groups follow first occurrence;
members retain their row order. Keys retain their value type.
The [grouping reference](values.md#stable-record-groups) describes the callable
form, common reader and supported keys.

Positional comprehensions create new one-based positions. To retain explicit
keys, `entries(a)` exposes present keyed cells as positional `key`/`value`
records. `zip_exact(a, b)` pairs keyed arrays only when their ordered axes and
present full keys match, returning two-element tuple cells. Equal lengths alone
are insufficient. Two sparse arrays with keys `[1, 4]` and `[1, 3]` cannot be paired
by silently matching their second entries.

The [keyed helper reference](values.md#keyed-entries-and-exact-pairing) describes
the callable forms and common reader. Both use the existing keyed value
representation and retain equal empty axes during pairing. Domain restrictions
on the allowed axes or missing cells remain host policy.

## Match neighbouring word occurrences

For a structural example, extend the word host with handles identifying
individual occurrences and their source positions. In `ash apple apple birch`,
the two `apple` handles are distinct. The host supplies `has_next`, `next_word`
and a `same_text` guard that compares spellings:

```text
let matches = match {
  from word in (input | filter(has_next));
  bind following = next_word(word);
  where same_text(word, following);
};

let rows = matches | project(m, {
  "word": m.word | text | one,
  "position": m.word | position | one
});
report {"matches": rows, "count": length(rows)};
```

Using one-based word positions, the intended report contains the occurrence
`apple` at position 2 and a count of 1. Captures retain both native handles.
Projecting a report produces scalar fields while the native records keep
occurrence identity.

`from` enumerates candidates, `bind` requires one native value per candidate
and `where` guards the match. Later clauses see earlier bindings. Zirql owns
that scope and enumeration; the proposed word adapter supplies adjacency,
source positions and text comparison. An Unknown final guard fails at its
query location. The host retains any separate incompleteness reasons. See
[the callable native match contract](host-integration.md#native-match-adapter).

## Explain a connection with a bounded path

An occurrence host can define a `next_word` edge between adjacent words. For
`ash apple birch`, a path from `ash` to `birch` needs two edges:

```text
filter(prefix("ash"))
| paths(next_word, to=prefix("birch"), max_depth=2)
```

With that adapter, the result has one witness with steps `[ash, apple, birch]`, including
both endpoints. A bound of one gives no witness for that pair. The selected
direction is one deterministic shortest witness per reachable source/sink pair
within the explicit edge bound, rather than an enumeration of every path.

The host supplies edges and occurrence identity. Zirql owns bounded traversal
and witness selection, preserving host uncertainty. A matching source has a
zero-edge witness, and a final Unknown target refuses. The query is read-only.
See [the callable path contract](native-programs.md#explain-native-connections).
[Semantic model integrations](semantic-models.md) explains why a
different host can give the same language constructs different domain meanings.
