# Zirql

Zirql is a Rust library for bounded query pipelines over host-native values.
Zirql sequences filtering, head, count, stable unique, string tally and typed
host stages. Hosts parse their syntax, evaluate predicate leaves and execute
native navigation and projections. The common text reader supports
`filter(predicate)`, `head(nonnegative integer)` and terminal `count`.

The intended consumers are [Zirium](https://github.com/zayenz/zirium), for MLIR,
and [Zincite](https://github.com/zayenz/zincite), for MiniZinc. The native
controls demonstrate borrowed source selections, document-owned operation
handles and a host that projects native rows while retaining mutable context.
Consumer integrations, programs and comprehensions are later tasks.
The crate name is reserved by the yanked `0.0.0-prerelease` version. Consumer
integrations use verified Git revisions and, when packaging requires one,
verified prereleases. License: MIT OR Apache-2.0.

## A borrowed source query

This host accepts `prefix("text")` leaves and returns borrowed words. The host
chooses the initial selection and renders the result after evaluation.

```rust
use std::ops::Range;
use zirql::{
    Budget, EvaluationError, Host, Limits, Located, Location, NativeInput, NativeValue,
    QueryError, Truth, evaluate, parse_pipeline,
};

struct Words;

impl<'a> Host<&'a str, (), Located<String>> for Words {
    type Error = QueryError;
    type Key = ();

    fn evaluate_leaf(
        &self,
        prefix: &Located<String>,
        word: &&'a str,
        _context: &mut (),
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<QueryError>> {
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
# Ok::<(), EvaluationError<QueryError>>(())
```

## Parsing, lowering and native results

`parse_pipeline` reads pipes and the three common stages. Predicate precedence
is `not`, `and`, then `or`; parentheses group predicates. The leaf callback
consumes a whole host predicate, validates its arguments and returns a typed
payload. `ArgumentReader` keeps original UTF-8 byte offsets and offers
identifier, integer and quoted-string helpers. Hosts can read other argument
forms through `remaining` and `advance`. Parsing is read-only. Common stage
names and Boolean words are reserved in this syntax.

Typed frontends construct `Located<Stage<Leaf, HostStage>>` values and call
`Pipeline::new`. `Stage::Host` retains a typed native payload and the original
stage range. The common reader returns `Pipeline<Leaf>` with no host-stage
syntax. Count and tally end executable pipelines; lowering rejects a stage
after either result before evaluation. Query locations are half-open byte
ranges in the original query. Hosts retain their source coordinates and any
document identity needed to interpret them.

Evaluation takes explicit `NativeInput<Item, Context>` and returns
`NativeResult<Item, Context>`. The core moves rows and context without requiring
`Clone`, `Send`, `Sync` or `'static`. Context survives streams, `Count(usize)`
and `Tally(BTreeMap<String, usize>)`. Stable unique retains the first row for
each host identity key and preserves input order. Tally consumes string keys
and returns their counts in sorted key order. Hosts own rendering.

`Host<Item, Context, Leaf, HostStage>` declares a native `Error` type and an
identity `Key: Eq + Hash`. Its hooks receive the retained mutable context and
active budget:

- `prepare_stage` checks stream-kind applicability before every stage, even on
  an empty stream. It may update metadata such as document selection. Its
  default accepts filter, head and count; hosts explicitly enable new stages.
- `evaluate_stage` receives the typed host payload and moves the input rows
  into its output stream. Hosts check known sizes and charge scans before
  allocating; the core checks the returned stream ceiling.
- `prepare_filter_item` prepares a row once before its Boolean tree. Its
  default charges one row visit. Hosts can replace that preparation when their
  native admission already owns the work.
- `evaluate_leaf` evaluates a typed predicate. `identity_key` supplies unique
  keys; `tally_key` consumes a row and can move an owned string into the tally.

Filter keeps only `Truth::True`. Negation preserves `Unknown`; `and` stops at
`False`, and `or` stops at `True`. Remaining operands determine whether the
result is known or Unknown. Hosts use Unknown for incomplete predicate
knowledge. Evaluation and host hooks return `EvaluationError<Host::Error>`:
`Core(QueryError)` retains located core failures, and `Host(error)` retains
native errors intact, including distinct input and model-file coordinates.
Native errors need no error trait or serialization.

## One request budget

The caller chooses `Limits` and owns one mutable `Budget` for input admission,
evaluation and nested host calls:

- Work charges one unit per admitted item, stage visit and visited predicate
  node. Default filter preparation and each unique/tally row also charge one
  unit. Hosts charge scans or traversal with `Budget::charge` before doing the
  work. Unique/tally key hooks charge additional work beyond the core's row
  visit; a string key of length `n` can charge `n.saturating_sub(1)` to preserve
  a total charge of `n.max(1)`. Successful and failed calls retain spent work.
- Items is a per-stream ceiling. `NativeInput::collect` checks it and charges
  work before each push. `NativeInput::from_admitted` accepts rows whose host
  scan/admission work was already charged to the same budget before allocation;
  it checks the ceiling without charging again. Host-stage output receives the
  same size check without readmission. Hosts use `Budget::check_items` before
  their own known allocations. This slice has no aggregate expansion counter.
- `Pipeline::new` and `parse_pipeline` count one level for the pipeline and each
  active predicate node, including groups. Stages add no depth. Their supplied
  allowances and the active budget's allowance are capped at `MAX_NESTING` (64).

`Pipeline::with_group_not_nesting` supports frontends such as Zincite that
count only written Group and Not wrappers. Pipeline, leaf and flat And/Or
nodes consume no logical depth. Zero accepts a flat Boolean tree, one accepts
one wrapper, and 64 accepts 64 wrappers with the frontend's precedence vectors.
Both lowering and evaluation walk predicates iteratively. A separate cap of
64 active `evaluate` calls bounds host reentry even when logical depth is zero.

Hosts pass explicit nested input and the same active budget to `evaluate`.
Nested calls retain active logical depth and remaining work. Evaluation
restores active depth and call state on success or error. Hosts choose parsing
and execution allowances suitable for their request.

Read the [base brief](.zdev/base/brief.md) for the integration order and testing
level. [TASKS.md](.zdev/base/TASKS.md) is generated by zdev. The GitHub repository
is public. Independently verified prereleases may support consumer packaging.
