# Comprehensions with a shared evaluator

Zincite needs comprehensions that read naturally as MiniZinc. Zirium constructs
JSON-shaped reports and exposes a Python query API. Their surface forms can
differ while sharing binding scope, evaluation order, checked operations and
resource accounting.

Read this design when working on value expressions, either comprehension
frontend, structured reports or constructed data. The base brief owns framework
boundaries. Zincite's `docs/query-comprehension-contract.md` owns its current
proposal for model-backed data edits and must be read before that integration.
The positional report examples work through both host frontends. Keyed arrays
and data-edit examples describe future host work.

## Equivalent positional reports

For a projected string sequence `labels = ["alpha", "beta"]`, Zincite's
MiniZinc surface can build records:

```text
source labels = query("names");
report [(name: n, label: "data:" ++ n) | n in labels];
```

Zirium keeps its existing binding and object notation:

```text
labels = names;
[{"name": n, "label": "data:" ++ n} for n in labels] | json
```

`++` maps to the same checked concatenation operation here. Zirium's extension
uses `[expression for binder in source if condition]`, with repeated `for`
clauses for dependent generators. Existing plain array literals remain valid.
Object values and strings inside comprehensions are expressions; existing
interpolated JSON literals retain their established behavior outside them.
Field labels are static strings, and bare bound strings are values. Do not
silently reinterpret old programs or give a native operation JSON identity.

Both examples construct the same ordered pair of records as native values.
Rendering differs: Zirium may produce ordinary JSON, while Zincite retains its
tagged report envelope and completeness information. The shared-value check
compares data before rendering, so shape metadata is not lost through JSON.

## Normalized generator mechanics

Represent a comprehension as its collection constructor, ordered generator
clauses, optional filters, head expression and optional key expression. Give
every clause and expression a surface-source range. No frontend runs a second
evaluator after lowering.

- A finite `in` source yields values in its declared order. Arrays yield values;
  explicit indexing is available for callers that need keys.
- Later sources and filters see earlier bindings. A generator's own bindings
  are visible in its filter and head, not its source. The rightmost generator
  varies fastest. Preserve these rules for repeated Zirium `for` clauses.
- MiniZinc multiple-name generators lower to repeated iteration over the same
  source; `_` discards its binding. A single-name `=` generator binds one value
  per preceding environment. A surface that omits a form cannot change its
  meaning when lowering another form to it.
- Bindings are immutable; duplicate names, shadowing disallowed by the host,
  forward references and undefined names receive located errors.
- Filters require known Booleans. False skips the head; Unknown is a refusal
  at the condition, including under negation. Report incompleteness cannot
  silently drop uncertain generator candidates.
- Validate the supported forms and incompatible types before evaluation.
  At runtime, skipped branches consume no arithmetic or missing-key work.
  A skipped branch does not hide unsupported syntax.

The evaluator owns finite ranges, iteration, scope, short-circuit conditionals,
access, construction and checked operations. Host lowering declares the allowed
forms. Zincite retains the closed subset in its existing contract: no model
calls, decision values, recursive expressions or general MiniZinc execution.

## Scalars and constructors

Share signed 64-bit integers, finite binary64 floats, Booleans and strings.
Integer arithmetic, range endpoints and index arithmetic check overflow.
Division checks zero divisors and the `i64::MIN / -1` family. Integer-to-float
promotion requires exact representation. Nonfinite values/results and nonzero
decimal values or arithmetic results rounded to zero are errors for the
Zincite-compatible subset. Keep expression equality/type rules explicit;
there is no string-to-number coercion or enum-member identity from spelling.

Positional sequences preserve generation order. Tuples and records preserve
their declared positions/labels; record labels are not scalar values. A
string-keyed report map rejects duplicate keys rather than overwriting them.
Typed sets remove duplicate identities and use their supported type order.
Zincite excludes string sets because its MiniZinc target cannot serialize them;
a host that permits them must define an order and renderer explicitly.

Host-native handles are admissible in native selection results and structured
results where the host already permits them. Scalar comprehensions require
explicit projections. They cannot use handle debug text as equality or a key.
Nominal enum members carry a host-owned enum/member identity and declared order;
Zincite resolves them with existing model facts. Zirql does not load models.

## Keyed arrays and target checks

Keep positional arrays, explicitly keyed arrays and nested arrays distinct.
Generic keyed construction checks key arity, compatible key identities,
duplicate keys and established axes. Iteration is row-major in the explicit
axis order. Empty axes require an explicit axis description when inference
cannot establish their rank or identity.

Zincite additionally requires contiguous integer/member axes, rectangular
coverage, compatible target declaration types and coherent candidate shapes.
Its adapter validates these obligations after construction and before report
or `.dzn` serialization as required by the consumer contract. Equal cardinality
is not equal index identity. A filter cannot silently compact a keyed array.
Zirql supplies construction data and reusable shape checks when both consumers
actually need them; the MiniZinc target policy remains with Zincite.

Queries that need keys use explicit key projections/access, for example the
existing proposed Zincite `index_set(a)` helper. Host helpers run with the same
budget and bounded operation whitelist. They do not enable arbitrary callbacks.

## Budgets, diagnostics and data effects

One active budget covers generator candidates before filters, expression visits,
nested construction, copies and rendering. Check products and known byte sizes
before allocation. Charge repeated expansion and copying again. Maintain the
host's existing per-stream ceiling alongside the aggregate request allowance;
neither nested queries nor a frontend conversion may reset either applicable
counter. Exhaustion fails the request rather than truncating its result.

Keep source locations through lowering. Invalid syntax or operators point to
the surface expression; uncertain enum/key/source facts retain the host's file
location. Generated values never acquire fabricated input ranges.

Construction itself is pure. Zincite's adapter converts native constructed
values into source replacements, checks target ownership and coherent axes,
prepares one atomic batch and reparses the complete candidate. Its existing
comment/directive and complete-before-write policy applies. The shared core
neither writes files nor guesses dependent reference mappings. A doubled data
instance explicitly remaps entities and every dependent collection; doubling
numeric values alone is not that operation.

## Focused evidence

Use one equivalent positional report through both frontends; dependent
generators and a filter must make ordering observable. Check one scope refusal,
a skipped-runtime-error control, a checked arithmetic failure and a pre-filter
expansion limit. Keyed-array work adds one explicit integer/enum multidimensional
control and a missing-cell refusal. Data work adds the public doubled-instance
control and a failed multi-assignment atomic-write check. Reuse these controls
across consumers rather than creating a full Cartesian syntax matrix.
