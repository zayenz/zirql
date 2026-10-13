# Language examples

A function report needs to keep names and counts together. A capacity report
needs to compare matching cells, even when the totals look sufficient. These
examples explain the language being designed for those queries.

The programs on this page are design sketches. They explain the planned shape
of the language; current parsers do not execute them. Syntax, supported subsets
and delivery can change. For callable behavior today, use the
[current Rust guides](getting-started.md).

## Short queries in the terminal

The proposed command forms accept a short query directly:

```sh
zirium 'filter(op("func.func")) | project(f, {"name": f | attr("sym_name") | one, "n": f | children | count}) | json' input.mlir
zincite-query --program 'source d = literal("demand"); report sum(d);' data.dzn
```

Zirium would keep its inline query argument. Zincite would accept explicit
`--program` text alongside a file form, preserving its legacy positional query.
Inline and file programs would use the same evaluation, native results,
diagnostics and limits. A file is useful for reuse and editing a longer report.

## Correlate fields with one native subject

Suppose an MLIR module has functions `double`, `mix` and `plain`, containing 2, 3
and 1 body operations respectively. The first two each contain one `arith.addi`.
A report should keep each function's name and counts together.

```text
param operation_name: string;
param minimum_ops: int;

let threshold = require(minimum_ops >= 0, minimum_ops,
                        "minimum_ops must be nonnegative");
let functions = input | filter(op("func.func"));
let rows = functions | project(f, {
  "name": f | attr("sym_name") | one,
  "operations": f | children | subtree | count,
  "matches": f | children | subtree
               | filter(op(operation_name)) | count
});
let selected = [r for r in rows if r.operations >= threshold];

report {
  "functions": selected,
  "total_operations": sum([r.operations for r in selected]),
  "total_matches": sum([r.matches for r in selected])
};
```

With `operation_name = "arith.addi"` and `minimum_ops = 2`, the intended result is:

```json
{
  "functions": [
    {"name": "double", "operations": 2, "matches": 1},
    {"name": "mix", "operations": 3, "matches": 1}
  ],
  "total_operations": 5,
  "total_matches": 2
}
```

`project(f, {fields})` would bind each native function to `f`, evaluate fields in
written order and produce one record per input item. Input order and duplicates
remain observable. The subject binding is scoped to the projection.
`one` would require exactly one value. A missing or ambiguous name would fail.
Two distinct functions sharing a name would still produce two rows.

Zirql would own the projection and composition rules. Zirium would own `op`,
`attr`, navigation and operation identity. Only explicit projections such as
`attr` convert native facts into constructed scalar values.

## Required parameters

A reusable report can declare required `int`, `string` and `bool` inputs with
`param`. The candidate CLI spelling is repeated `--param name=value`, for example
`--param operation_name=arith.addi --param minimum_ops=2`. Library callers would
supply typed values directly.

The intended rules reject missing, duplicate, extra or incompatible inputs
before domain execution. Strings stay verbatim, integers are checked decimal
`i64`, and Booleans are exactly `true` or `false`. Parameters would not perform
query-text substitution. Parameter defaults and named result schemas are outside
this initial design.

## Capacity at each position

The candidate reducers `sum`, `length`, `forall` and `exists` operate on evaluated
arrays. Integer sums are checked. Quantifiers accept known Booleans. Starting
with eager arrays keeps errors and budget spending consistent with ordinary
comprehensions; lazy generator reduction would need a separate contract.

`require(condition, value, message)` would validate all operands, then evaluate
the condition. On true it evaluates the value; on false it fails at the
requirement with the message. Only the selected value is evaluated.

For MiniZinc data `demand = [2, 3, 4]` and `capacity = [6, 8, 5]`, scaling demand
by 2 gives `[4, 6, 8]`. Totals fit (18 ≤ 19), but the third cell does not.
The following proposed fragment makes that distinction explicit:

```text
let aligned_capacity = require(
  index_set(demand) = index_set(capacity), capacity,
  "demand and capacity must have the same index set"
);
let scaled = [2 * d | d in demand];
let rows = [
  (index: i, demand: scaled[i], capacity: aligned_capacity[i])
  | i in index_set(demand)
];
report (
  totals_fit: sum(scaled) <= sum(aligned_capacity),
  cells_fit: forall([r.demand <= r.capacity | r in rows]),
  violations: [r | r in rows where r.demand > r.capacity]
);
```

This uses MiniZinc-style expression notation within a candidate Zincite program.
The intended report succeeds with `cells_fit = false` and one violation.
Insufficient capacity is report data. A failed requirement, invalid access or
arithmetic overflow is an execution error. Host rendering would retain Zincite's
completeness and limitations information.

## Group rows and align keyed cells

`group_by(rows, "matches")` would return records with `key` and `items`. Applied
to the function rows above, it would put `double` and `mix` in the group keyed by
integer 1, then `plain` in the group keyed by integer 0. Groups follow first
occurrence; members keep their row order. Keys retain their value type.

Positional comprehensions construct new one-based positions. They cannot
preserve arbitrary integer or nominal keys merely by iterating the values.
The candidate `entries(a)` helper would expose keyed cells.
`zip_exact(a, b)` would align two keyed arrays only when their ordered axes and
present full keys match. Equal lengths alone are insufficient, especially for
sparse arrays. Missing or incompatible keys should fail before cell arithmetic.
These helpers would build on the [current keyed value representation](values.md#keyed-arrays).

## Match operations and retain their operands

A structural query should retain the operation and operand handles long enough
to ask further questions about them. This proposed match finds additions using
the same SSA value twice:

```text
let matches = match {
  from add in (input | filter(op("arith.addi")));
  bind lhs = operand(add, 0);
  bind rhs = operand(add, 1);
  where same_value(lhs, rhs);
};

let rows = matches | project(m, {
  "function": m.add | root(op("func.func")) | attr("sym_name") | one,
  "operation": m.add | names | one
});
report {"matches": rows, "count": length(rows)};
```

`from` would enumerate native candidates, `bind` would retain one value per
candidate and `where` would guard the match. Later clauses see earlier bindings.
Each match returns a native capture record; field access such as `m.add` retains
the operation type. The final projection constructs scalar report rows.

For an addition of `%x` to itself in `double`, the intended result contains one
match. Equal printed SSA names in different scopes do not establish equality.
Two results of one multi-result operation also remain distinct.
Zirium would supply operand lookup and SSA identity; the base would own clause
scope, enumeration and budget propagation.

An Unknown required guard needs a located refusal with the host's reason.
Silently discarding it would hide uncertainty. This observation also says
nothing about whether rewriting the addition is legal.

## Explain a connection with a bounded path

The path direction is one deterministic shortest witness per reachable
source/sink pair within an explicit edge-depth bound. A host would supply edges
and native identity; the base would own bounded traversal and selection of the
witness. It would preserve host uncertainty and diff-side rules.

For edges `a → b`, `b → c` and `a → c`, a bound of one can report the direct
`a → c` witness. Removing that edge requires a bound of at least two to expose
`a → b → c`. The query would not enumerate every possible path. Edge meanings
and the surface spelling still need concrete host choices.
