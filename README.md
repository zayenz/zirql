# Zirql

Zirql is a Rust library for bounded query pipelines and programs over host-native values.
Zirql sequences filtering, head, count, stable unique, string tally and typed
host stages. Hosts parse their syntax, evaluate predicate leaves and execute
native navigation and projections. The common text reader supports
`filter(predicate)`, `head(nonnegative integer)` and terminal `count`.

The intended consumers are [Zirium](https://github.com/zayenz/zirium), for MLIR,
and [Zincite](https://github.com/zayenz/zincite), for MiniZinc. The native
controls demonstrate borrowed source selections, document-owned operation
handles and a host that projects native rows while retaining mutable context.
Programs add immutable bindings, grouped expressions, ordered set operands,
native records and explicit emission policies. Hosts supply these operations
through typed hooks. Comprehensions and consumer program integrations remain
later tasks. License: MIT OR Apache-2.0.

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
identity `Key: Eq + Hash`. Evaluation hooks receive the retained mutable
context and active budget; work-cost hooks declare core charges:

- `stage_work(items, context)` returns the work charged at the stage location
  before `prepare_stage`. It receives the current input rows and immutable
  context and defaults to one unit. A host can return `items.len().max(1)` to
  charge by input size, including one unit for an empty stream.
- `collection_item_work()` returns the work charged at the stage location before
  each `identity_key` or `tally_key` call. It defaults to one unit. A host can
  return zero when its stage or admission charge already covers row visits.
- `prepare_stage` checks stream-kind applicability before every stage, even on
  an empty stream. It may update metadata such as document selection. Its
  default accepts filter, head and count; hosts explicitly enable new stages.
- `evaluate_stage` receives the typed host payload and moves the input rows
  into its output stream. Hosts check known sizes and charge scans before
  allocating, and admit newly produced results in program mode; the core checks
  the returned stream ceiling without admitting rows a second time.
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

- Work charges one unit per admitted item and visited predicate node. Stage
  visits use `stage_work`; unique/tally rows use `collection_item_work`. Both
  default to one unit, as does default filter preparation. Hosts charge scans
  or traversal with `Budget::charge` before doing the work. Unique/tally key
  hooks charge work beyond their declared row cost; with the default row cost,
  a string key of length `n` can charge `n.saturating_sub(1)` to preserve a total
  charge of `n.max(1)`. Weighted stage costs and zero collection-row costs let
  hosts preserve input-length stage accounting without another row charge.
  Successful and failed calls retain spent work.
- Items is a per-stream ceiling. `NativeInput::collect` checks it and charges
  work before each push. `NativeInput::from_admitted` accepts rows whose host
  scan/admission work was already charged to the same budget before allocation;
  it checks the ceiling without charging again. Host-stage output receives the
  same size check without readmission. Hosts use `Budget::check_items` before
  their own known allocations.
- `Pipeline::new` and `parse_pipeline` count one level for the pipeline and each
  active predicate node, including groups. Stages add no depth. Their supplied
  allowances and the active budget's allowance are capped at `MAX_NESTING` (64).

`Pipeline::with_group_not_nesting` supports frontends such as Zincite that
count only written Group and Not wrappers. Pipeline, leaf and flat And/Or
nodes consume no logical depth. Zero accepts a flat Boolean tree, one accepts
one wrapper, and 64 accepts 64 wrappers with the frontend's precedence vectors.
Both lowering and evaluation walk predicates iteratively. A separate cap of
64 active pipeline/expression calls bounds host reentry even when logical depth
is zero.

Hosts pass explicit nested input and the same active budget to `evaluate`.
Nested calls retain active logical depth and remaining work. Evaluation
restores active depth and call state on success or error. Hosts choose parsing
and execution allowances suitable for their request.

## Native programs

Hosts lower their syntax into `Program<Stage, SetOperator, Emitter>` and located
`Expression` values. `Statement::Bind` defines an immutable native value;
`Statement::Evaluate` evaluates and discards it or requests the supplied implicit
emitter. Each statement starts from the original top-level input. Earlier
statements receive a native copy through the host's `copy_value` hook; the final
statement receives the original input by move. Initial `Bindings::new` values
reserve their names. `Bindings::get(name, range)` exposes read-only native lookup.

Expressions support current `Input`, `Binding`, `Group`, `Sequence` of typed
stages/emitters, binary `Set` and ordered `Record` fields. Group passes current
input. Set evaluates left before right; record fields run in written order.
Independent operands receive charged native input copies, with original input
moved into the final operand. The host chooses set identity/order and constructs
its native record from the evaluated field vector. No native value needs `Clone`,
`Send`, `Sync`, `'static` or serialization.

For example, this lowering binds input selections and requests a native record
containing the selection and a host-defined count. The host supplies the count
stage and report emitter. The function returns bindings for native inspection.

```rust
use zirql::{
    Bindings, Budget, EvaluationError, ExpressionKind, ExpressionStep, Located,
    Program, ProgramHost, Statement, evaluate_program,
};

fn report<C, H: ProgramHost<C>>(
    host: &H, input: H::Value, context: &mut C, budget: &mut Budget,
    count: H::Stage, emitter: H::Emitter,
) -> Result<Bindings<H::Value>, EvaluationError<H::Error>> {
    let count = Located::new(ExpressionKind::Sequence {
        input: Box::new(Located::new(ExpressionKind::Binding("selected".into()), 40..48)),
        steps: vec![Located::new(ExpressionStep::Stage(count), 51..56)],
    }, 40..56);
    let fields = vec![
        (Located::new("selection".into(), 20..29),
         Located::new(ExpressionKind::Binding("selected".into()), 31..39)),
        (Located::new("count".into(), 40..45), count),
    ];
    let program = Program::new(vec![
        Located::new(Statement::Bind {
            name: Located::new("selected".into(), 0..8),
            expression: Located::new(ExpressionKind::Input, 11..16),
        }, 0..16),
        Located::new(Statement::Evaluate {
            expression: Located::new(ExpressionKind::Record(fields), 20..57),
            implicit_output: Some(emitter),
        }, 20..57),
    ], 0..57);
    evaluate_program(&program, host, input, context, Bindings::default(), budget)
}
```

`ProgramHost<Context>` supplies native `Value`, `Stage`, `SetOperator`, `Emitter`
and `Error` types. Its hooks copy values, evaluate stages, combine set operands,
construct records and observe emission requests. Every hook receives the same
mutable host context and active budget. Stage hooks also receive bindings;
native map/key/selector stages call `evaluate_expression` with relative input,
those bindings, unchanged context and that budget. Existing pipelines run through
`evaluate` with a context wrapper containing `&mut Context`. No query restart or
fresh nested budget is needed.

The runner preflights the complete program before native copies, stages or
emissions. It rejects core-visible undefined/forward reads, duplicate definitions,
redefinitions of initial names, structural nesting overflow and steps after a
terminal emitter at their query ranges. Hosts must first validate names inside
opaque payloads, pure binding/selector/constructor operands, supported operations,
duplicate record labels and value transitions, including empty streams. Core
preflight cannot inspect a host payload. Runtime host failures keep their native
error type; core failures keep their location.

An explicit `Emit` runs at its written position. `EmissionPolicy::PassThrough`
retains the value for later steps; `Terminal` ends the expression. A final
explicit emit, including through Group, suppresses that statement's implicit
output. The core requests emission without rendering or promising rollback.
Library hosts choose whether to collect native requests or invoke callbacks as
they run. CLI hosts buffer all requested output and commit stdout only after the
whole request succeeds. An earlier library callback or host effect can remain
observable after a later failure.

## Explicit aggregate limits

`Budget::for_program(ProgramLimits { work, results, nesting, stream_items })`
enables one aggregate allowance for produced/copied native entries across the
request. `Budget::new(Limits)` keeps its existing work charges and per-stream
ceiling; `charge_results` is inactive for that legacy budget. Hosts choose finite
program allowances explicitly. The core provides no default limits.

One native row or scalar consumes one result unit. A record/map constructor
container consumes one, and each field/map entry consumes one. Nested values
consume their own units when produced. Thus a count consumes one, and a tally
consumes one container plus two units per distinct key: its entry and count.
Moving admitted rows through filter/unique or moving a value into a binding does
not admit them again. Repeated value, record, report and emission-snapshot copies
consume new units. Raw source/model scan nodes consume work rather than results.

Producers call `charge_results(units, location)` before allocation and check known
sizes with `check_items`. `NativeInput::collect` admits each row before pushing;
`from_admitted` accepts rows already charged to the same active budget. Returned
streams receive ceiling checks without readmission. Core count/tally production
charges new scalars and map entries. Host-stage producers and `copy_value` own
their native charges. Core record staging admits its container/field entries and
charges copied label bytes before allocation; `construct_record` can move that
vector directly without another charge. Additional host materialization needs
its own admission. Hosts charge copied string/label and rendered bytes to work
before known allocations. Result exhaustion fails the request without truncation.

Program budgets charge one work unit per visited expression, plus copied binding
name and record-label bytes. Input/Binding/Sequence wrappers add no logical
nesting; Group/Set/Record each add one active level. Opaque nested selectors and
pipeline predicates share the same remaining allowance and active call cap.
Success and failure restore active depth/call state and retain spent work/results.

Read the [base brief](.zdev/base/brief.md) for the integration order and testing
level. [TASKS.md](.zdev/base/TASKS.md) is generated by zdev. The GitHub repository
is public. Independently verified prereleases may support consumer packaging.
