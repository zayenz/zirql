# Pipelines

A pipeline filters or transforms an ordered stream of native items. The host
provides those items and implements domain predicates. Zirql runs the stage
sequence and combines predicate results.

## The common text reader

`parse_pipeline` accepts these shared stages:

```text
filter(prefix("a")) | head(1) | count
```

The host's parser callback reads `prefix` and validates its arguments.
`filter`, `head` and `count` are the common reader's stage vocabulary. `head`
accepts a nonnegative integer; `count` ends the pipeline.

Predicates support `not`, `and`, `or` and parentheses. Precedence is `not`, then
`and`, then `or`. Evaluation short-circuits. An Unknown predicate does not become
true when negated; hosts that have only Boolean answers can return only
`Truth::True` and `Truth::False`.

The reader keeps UTF-8 byte offsets through `ArgumentReader`. Its helpers read
words, integers and quoted strings. A host may consume additional argument
forms with `remaining` and `advance`. Parsing has no domain effects.

## Typed pipelines

A host frontend can also construct `Located<Stage<Leaf, HostStage>>` values and
call `Pipeline::new`. This path exposes `Unique`, string `Tally` and typed
`Host` stages as well as filtering, head and count. The common text reader does
not supply their syntax.

`Unique` keeps the first item for each host identity key, preserving input order.
`Tally` counts host-projected string keys and returns counts in sorted key order.
Both count and tally are terminal. A stage after either is rejected while
lowering, before evaluation.

`head(1)` deliberately takes the first item. The proposed `one` operation has a
different purpose: it would require exactly one item. See the
[language direction](language-direction.md).

## Results retain their input context

`evaluate` accepts `NativeInput<Item, Context>` and returns
`NativeResult<Item, Context>`. A result holds a stream, `Count(usize)` or a string
tally, together with the retained context. The evaluator does not require
items to implement `Clone`, `Send`, `Sync` or `'static`.

For a source query, an item can borrow its text. For an operation query, an item
can be a handle owned by the retained document. The host decides what identifies
an item, how long the handle remains valid and how to render it. Equal printed
names need not mean equal domain objects.

## Programs and constructed data

[Native programs](native-programs.md) add bindings, records and ordered
statements. [Constructed values](values.md) add scalar arithmetic and
comprehensions. These callable APIs take located Rust representations. A host
frontend must supply program and value syntax.

All nested calls use the same active [budget](diagnostics-and-limits.md).
A nested query does not gain a new allowance merely because it has a new input.
