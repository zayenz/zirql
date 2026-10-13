# Zirql

[Documentation and examples](https://zayenz.se/zirql/) cover the language and
embedding Zirql in a Rust project.

Zirql is a Rust library for bounded query pipelines, native programs and
constructed scalar, nominal and keyed comprehension values.
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
through typed hooks. The pure value evaluator accepts a located typed expression
without a parser, consumer adapter or renderer. License: MIT OR Apache-2.0.

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

let source = String::from("birch apple ash apple");
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
Consecutive left-associated set operators run iteratively. Each operand receives
independent current input; grouping and right-nested sets retain their nesting
limits.
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
nesting; Group/Record and each set chain add one active level. Consecutive
left-associated Set nodes share that level; grouped and right-nested sets add
levels. Opaque nested selectors and pipeline predicates share the same remaining allowance and active call cap.
Success and failure restore active depth/call state and retain spent work/results.

## Constructed values and positional comprehensions

Hosts lower their expression syntax to `ValueExpression`, whose nodes retain
query byte ranges. The evaluator returns owned `ConstructedValue` data: fixed
Booleans, signed `i64`, finite `f64`, strings, positional arrays, tuples, ordered
records and homogeneous Boolean/integer/float sets. Arrays can contain different
value types and keep positive `1..n` positions in generation order. Tuples and
records retain declaration order. Sets deduplicate by typed identity and use
`false` before `true` or numeric order. Empty arrays and sets are valid.

Zincite's MiniZinc-shaped reports and Zirium's JSON-shaped comprehensions use
this evaluator. Both frontends project native names and retain constructed
values for inspection before their own renderers run.

A normalized comprehension chooses `ValueConstructor::Array` or `Set`, supplies
ordered `ValueGenerator` clauses and a located head. `In` yields source values;
use `IndexSet` and `ArrayAccess` when positions matter. Grouped binders evaluate
one source before installing any of their names, then vary the last binder
fastest. `Bind` evaluates one value per preceding environment. Later clauses
see earlier names. `None` discards a binding; `_` cannot be read. Duplicate
names, source/let shadowing and forward reads fail at their responsible ranges.
Filters and conditional conditions require actual fixed Booleans.

Development Git builds additionally expose `evaluate_value_with_host_validation`.
It accepts a trusted Rust host check of `(&ConstructedValue, &mut Budget,
&Location) -> Result<(), QueryError>` and runs it before each successful runtime
value reaches its parent. This lets Zincite reject incomplete rectangular arrays
before inline iteration converts them to positional values. The check runs
inside the existing evaluation guard with the same budget; skipped runtime
branches are untouched. `evaluate_value` retains generic sparse values and its
existing spending through a no-op check. This additive entry point is absent
from the published alpha.5 registry release; consumers need its pinned
development Git source until release work supplies a registry version.

The five budgeted functions support the host projection sequence:

1. `free_value_names` checks scope and declared facts, then returns distinct
   global names at first-use ranges. A host can use `ValueType::Unestablished`
   for a declared source whose projection is not yet available.
2. `constructed_value_type` checks projected values and discovers actual facts.
   `ArrayType::Positions` carries exact positions; `Elements` carries known
   possible element types when length and positions are not established.
3. `validate_value_expression` provides explicit preflight against those facts.
   It visits every branch, filter and head, including unchosen branches.
4. `evaluate_value` accepts `Bindings<ConstructedValue>` and performs its own
   complete actual-type validation before execution. It returns one value.
5. `copy_constructed_value` admits a binding or emission snapshot before
   allocating its recursive copy. Constructed trees have no deep `Clone`.

Hosts project only the requested globals, admit those values with the retained
budget, and move them into `Bindings::new`. Explicit preflight is useful before
projection and does not exempt later work. Each call spends its own allowance.
Hosts retain native handles, source coordinates and completion metadata. They
render the successful value with the same budget; constructed values do not
invent original-source coordinates.

This typed report keeps the input label order before any rendering:

```rust
use std::collections::BTreeMap;
use zirql::{
    ArrayType, Bindings, Budget, ConstructedValue as V, Located, Location,
    ProgramLimits, ValueConstructor, ValueExpressionKind as K, ValueGenerator,
    ValueType, constructed_value_type, copy_constructed_value, evaluate_value,
    free_value_names, validate_value_expression,
};

let expression = Located::new(K::Comprehension {
    constructor: ValueConstructor::Array,
    clauses: vec![Located::new(ValueGenerator::In {
        binders: vec![Located::new(Some("n".into()), 15..16)],
        source: Located::new(K::Name("labels".into()), 20..26),
        filter: None,
    }, 15..26)],
    head: Box::new(Located::new(K::Record(vec![
        (Located::new("name".into(), 2..6),
         Located::new(K::Name("n".into()), 8..9)),
    ]), 1..10)),
}, 0..27);
let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 1_000, nesting: 32, stream_items: 100,
});
let declared = BTreeMap::from([("labels".into(),
    ValueType::Array(ArrayType::Elements(vec![ValueType::String])))]);
let names = free_value_names(&expression, &declared, &mut budget)?;
assert_eq!(names[0].value, "labels");

let source = V::Array(vec![V::String("beta".into()), V::String("alpha".into())]);
let projected = copy_constructed_value(&source, &mut budget, Location::Source(0..14))?;
let ty = constructed_value_type(&projected, &mut budget, Location::Source(0..14))?;
validate_value_expression(&expression, &BTreeMap::from([("labels".into(), ty)]), &mut budget)?;
let bindings = Bindings::new([(Located::new("labels".into(), 20..26), projected)])?;
let value = evaluate_value(&expression, &bindings, &mut budget)?;
assert_eq!(value, V::Array(vec![
    V::Record(vec![("name".into(), V::String("beta".into()))]),
    V::Record(vec![("name".into(), V::String("alpha".into()))]),
]));
let snapshot = copy_constructed_value(&value, &mut budget, Location::Query(0..27))?;
assert_eq!(snapshot, value);
# Ok::<(), zirql::QueryError>(())
```

The subset includes checked sign, `+`, `-`, `*`, integer `div`/`mod`, floating
`/`, numeric comparisons, typed recursive equality and inequality, scalar set
membership, Boolean `not`/`and`/`or`, string and positional-array concatenation,
field/tuple/array access, `index_set`, inclusive integer ranges and ordered
conditional branches with a mandatory else. Integer division truncates toward
zero; the remainder has the dividend's sign. Integer overflow, zero division
and the `i64::MIN` with `-1` division/remainder cases fail. Mixed arithmetic and
comparisons require exact integer-to-binary64 promotion, including at the `i64`
boundaries. Nonfinite inputs/results and nonzero floating arithmetic rounded
to zero fail. Normalized `f64` literals retain no decimal spelling, so hosts
must reject a nonzero decimal literal that parses to zero during lowering.
Strings receive no numeric coercion.

`Reduce` supports the closed `ValueReducer::{Length, Sum, Forall, Exists}` over
fully materialized positional or keyed arrays. Length counts present values;
sum performs checked integer addition; the quantifiers scan all fixed Boolean
elements. Empty identities are `0`, `0`, `true` and `false`. Nested arrays are
not flattened, and sparse keyed arrays retain their present-cell meaning.
`Require { condition, value, message }` preflights both expressions and carries
a literal `Located<String>` message. A true fixed Boolean runs and returns the
value. False fails at the requirement with that message and skips the value's
runtime arithmetic, access, construction and host checks. Preflight still checks
all names and known types. See [values and comprehensions](docs/values.md#array-summaries-and-requirements)
for these contracts. These development Git forms are absent from alpha.5.

Known incompatible types and unsupported set elements fail before branch
choice. Missing fields and positions supply only unestablished validation
facts and fail if accessed at runtime. Boolean operations short-circuit and
conditionals execute only the chosen branch, so skipped missing access and
zero division remain unevaluated. Every referenced actual value is checked,
including nonfinite floats, duplicate record labels and malformed sets.
Resolved nominal members and explicitly keyed arrays are also callable native
values. Hosts supply `NominalTypeId` and `NominalMemberId` within one immutable
host context for the request, plus retained type/member names and declaration
order. Member equality and set deduplication use IDs; identical spelling does
not establish identity. Member sets use declaration order. The core rejects
contradictory names/order, incompatible enum types and member arithmetic.
Hosts retain member resolution, model execution, rendering and data-edit policy.

`KeyAxis` retains a dimension's integer/member type and ordered `ValueKey`s;
`KeyedEntry` retains a full key and native value. `ConstructedValue::KeyedArray`
is distinct from positional and nested arrays. `KeyedArrayType` retains axis
identities/rank and possible element types. All five value APIs validate and
copy these forms. Public native entries must be unique and in row-major axis
order, with the rightmost dimension fastest. `ValueExpressionKind::KeyedArray`
accepts located unordered key/value pairs and optional supplied axes.
`KeyedComprehension` shares the positional evaluator and generator clause chain,
with a key and head evaluated after filters. Integer/member scalar keys establish
rank one; nonempty flat tuples establish larger ranks. Inferred axes use numeric
or member declaration order; supplied axes preserve their explicit order.
Sparse entries and noncontiguous axes are valid. Zincite owns MiniZinc contiguous
axes, rectangular coverage, target compatibility and source diagnostics.

Array access accepts a matching scalar or one-component tuple key for rank one
and a matching tuple for larger ranks. Nested access remains nested. `index_set`
requires rank one and returns the established typed axis as a set in numeric or
declaration order, including typed empty axes. Empty native keyed arrays retain
all supplied axes. When a keyed comprehension produces zero entries,
`empty_axis_sources` can establish typed axes from integer/member arrays or sets.
Every fallback runs in the surrounding scope, outside this comprehension's
binders, including later sources skipped by an empty product. An empty range
retains integer identity, and an empty typed member set retains its nominal ID.
An empty positional array or untyped set establishes no key identity. Missing
fallback rank/types/axes receive a located refusal. Nonempty results do not
evaluate or apply fallback axes.

Value operations share the active `Budget`, including its remaining work,
aggregate results and nesting. Work pays expression/value/type visits,
validation metadata slots, known collection allocation, candidates before
filters, copied UTF-8 name/label/string bytes and compatibility/equality
comparisons. Results pay each new/copied scalar or container and each
collection/record entry slot. A nominal value or key pays one result unit;
each keyed axis, entry, axis-key slot and entry-key slot pays one, as does the
keyed container. Retained member/type names charge copied UTF-8 bytes, while
axis/type/position metadata, key comparisons and ordering movements charge work.
Children already admitted by evaluation move into slots without another value
admission. A snapshot scans its full size
and invariants, admits its result units and bytes, then builds the copy. Each
recursive expression/value/type walk retains active nesting. Both Result
outcomes restore nesting/call state; spending is never refunded.

Reducers retain operand materialization, candidate and copy charges. Sum and
both quantifiers charge one work unit per inspected element; length charges one
work unit for its metadata read and checked conversion. Each summary scalar
charges one aggregate result unit. Require preflight charges literal message
bytes; a false diagnostic charges the copied bytes before allocation. True
require moves its admitted child without a new result or copy charge. Empty
summaries still pay expression/preflight work and the scalar result unit.
Emission snapshots and rendering spend their own allowance again.

Known range sizes, grouped products, lengths and byte sums are checked before
allocation. A false filter cannot evade candidate admission. Limits fail at
the active query or supplied source location and return no partial value.
Legacy `Budget::new` keeps aggregate result accounting inactive, but these
value APIs still charge all work. Native work costs and per-stream limits
retain their existing contracts. There are no new default limits.

Read the [base brief](.zdev/base/brief.md) for the integration order and testing
level. [TASKS.md](.zdev/base/TASKS.md) is generated by zdev. The GitHub repository
is public. Independently verified prereleases may support consumer packaging.
