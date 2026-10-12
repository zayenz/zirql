# Zirql

Zirql is a Rust library for bounded query pipelines over host-native values.
The first slice supports `filter(predicate)`, `head(nonnegative integer)` and
terminal `count`. Hosts parse and evaluate their own typed predicate leaves;
Zirql supplies Boolean composition and ordered pipeline execution.

The intended consumers are [Zirium](https://github.com/zayenz/zirium), for MLIR,
and [Zincite](https://github.com/zayenz/zincite), for MiniZinc. The embedded
controls demonstrate borrowed source selections and document-owned operation
handles. Consumer integrations, programs and comprehensions are later tasks.
The crate is unpublished. License: MIT OR Apache-2.0.

## A borrowed source query

This host accepts `prefix("text")` leaves and returns borrowed words. The host
chooses the initial selection and renders the result after evaluation.

```rust
use std::ops::Range;
use zirql::{
    Budget, Host, Limits, Located, Location, NativeInput, NativeValue,
    QueryError, Truth, evaluate, parse_pipeline,
};

struct Words;

impl<'a> Host<&'a str, (), Located<String>> for Words {
    fn evaluate_leaf(
        &self,
        prefix: &Located<String>,
        word: &&'a str,
        _context: &(),
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, QueryError> {
        Ok(if word.starts_with(&prefix.value) {
            Truth::True
        } else {
            Truth::False
        })
    }
}

let query = r#"filter(prefix("a")) | head(1) | count"#;
let pipeline = parse_pipeline(query, |reader| {
    let name = reader.word()?;
    if name.value != "prefix" {
        return Err(QueryError::query(name.range, "expected prefix"));
    }
    reader.expect('(')?;
    let prefix = reader.quoted_string()?;
    reader.expect(')')?;
    Ok(prefix)
}, 8)?;

let source = String::from("birch apple ash");
let mut budget = Budget::new(Limits { work: 100, items: 10, nesting: 8 });
let input = NativeInput::collect(
    (), source.split_whitespace(), &mut budget, Location::Source(0..source.len()),
)?;
let result = evaluate(&pipeline, &Words, input, &mut budget)?;
assert_eq!(result.value, NativeValue::Count(1));
# Ok::<(), QueryError>(())
```

## Parsing, lowering and native results

`parse_pipeline` reads pipes and the three common stages. Predicate precedence
is `not`, `and`, then `or`; parentheses group predicates. The leaf callback
consumes a whole host predicate, validates its arguments and returns a typed
payload. `ArgumentReader` keeps original UTF-8 byte offsets and offers
identifier, integer and quoted-string helpers. Hosts can read other argument
forms through `remaining` and `advance`. Parsing is read-only. Common stage
names and Boolean words are reserved in this syntax.

Typed frontends construct `Located<Stage<Leaf>>` values and call `Pipeline::new`.
Both entry points validate predicate depth and reject any stage after `count`
before evaluation. Query locations are half-open byte ranges in the original
query; host-source locations refer to the host's retained source. Hosts retain
any document identity needed to interpret those ranges.

Evaluation takes explicit `NativeInput<Item, Context>` and returns
`NativeResult<Item, Context>`. Filter and head return a stream; count returns
`usize`. Streams preserve input order and duplicate identities. The opaque
context survives count results too. The core moves items and context without
requiring `Clone`, `Send`, `Sync` or `'static`, so borrowed slices and native
document handles remain usable. Hosts own navigation, identity checks,
limitations and rendering.

Filter keeps only `Truth::True`. Negation preserves `Unknown`; `and` stops at
`False`, and `or` stops at `True`. Otherwise the remaining operands determine
whether the result is known or Unknown. Hosts reserve Unknown for incomplete
predicate knowledge and return ordinary failures as located `QueryError`s.

## One request budget

The caller chooses `Limits` and owns one mutable `Budget` for input admission,
evaluation and any nested host calls:

- Work charges one unit per admitted item, stage visit, filter item visit and
  visited predicate node. Hosts charge extra scans or traversal with
  `Budget::charge` before doing the work. Already charged work is retained after
  both successful and failed evaluations.
- Items is a per-stream ceiling. `NativeInput::collect` checks it and charges
  work before each push; hosts check known sizes before their own allocations
  with `Budget::check_items`. Iterator work performed to produce an item is
  host-owned. This slice has no aggregate expansion counter.
- Nesting starts at one for a pipeline; each active predicate node adds one,
  including a parenthesized group. Stages add no depth. Parsing, typed lowering
  and evaluation enforce their supplied allowances, capped at `MAX_NESTING`
  (64). Evaluation restores active depth when it returns an error.

The input must be admitted against the budget passed to `evaluate`. A host
that calls `evaluate` from a leaf passes explicit nested input and that same
active budget. Nested evaluation retains the current depth and remaining work;
it never implicitly selects the whole document. Hosts choose parsing and
execution allowances suitable for their request.

Read the [base brief](.zdev/base/brief.md) for the integration order and testing
level. [TASKS.md](.zdev/base/TASKS.md) is generated by zdev. The GitHub repository
starts private; making it public is a later decision.
