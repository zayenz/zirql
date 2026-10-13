# Three programs for discussing the Zirql base language

These examples make the proposed extensions concrete enough to discuss. Every
program is a design sketch, not an executable acceptance test. Expected results
are derived from the inputs and proposed rules below; no current parser or
evaluator has executed these programs. The [base brief](../brief.md) owns settled
boundaries, and [the research](language-extensions.md) explains the alternatives.

The examples propose reusable language constructs in Zirql. Domain adapters
supply primitive operations and types. The MLIR examples use a candidate common
surface with `let`, `param`, `report`, and JSON-shaped record expressions. The
Zincite example keeps MiniZinc expression syntax and its source-binding wrapper.
Different frontend spellings lower to the same located constructs. This is not
a proposal to replace either consumer's existing grammar without compatibility
work.

The proposed task bundle in the brief selects explicit projection subjects,
typed external parameters, eager reducers and the separate `match` surface.
The alternatives below remain design rationale. Full and short programs must
use the same shared constructs; none of these examples requires a file merely
because it contains a binding or constructed value.

## Short command-line forms

These commands are also proposed syntax. They use literal inputs where a
reusable parameter declaration would add no value:

```sh
zirium 'filter(op("func.func")) | project(f, {"name": f | attr("sym_name") | one, "n": f | children | count}) | json' input.mlir
zincite-query --program 'source d = literal("demand"); report sum(d);' data.dzn
```

Zincite's umbrella `zincite query` command accepts the same `--program` option.
The explicit option preserves its legacy positional pipeline grammar. Zirium
keeps its existing inline query argument and implicit final emission. New
single-expression syntax needs no program wrapper or parameter preamble.

The parameterized file form of example 1 can use
`zirium --param operation_name=arith.addi --param minimum_ops=2 -f report.zirql input.mlir`.
The equivalent inline program accepts the same parameters. The brief owns the
typed argument decoding rules. File and inline source diagnostics identify the
actual query source; neither mode obtains a fresh budget for nested work.

## Shared candidate rules

These rules are recommendations for discussion, not new implementation authority.

| Form | Proposed meaning |
| --- | --- |
| `param name: type;` | Required typed caller input. Check missing, extra and incompatible inputs before host execution; no text substitution. |
| `let name = expression;` | Immutable binding with the existing scope and no-shadowing rules. |
| `selection \| project(subject, {fields})` | Bind each native input item to `subject`; evaluate fields in written order and emit exactly one constructed record. Preserve input order and duplicates. |
| `selection \| one` | Require exactly one projected value and extract it. Empty or multiple values fail; it never means take the first. |
| `sum`, `length`, `forall`, `exists` | Closed base operations over evaluated arrays. Integer sum is checked; quantifiers accept known Booleans. |
| `require(condition, value, message)` | Preflight all operands, then evaluate condition; on true evaluate value, on false fail with the supplied message at the requirement. The sketches use literal messages. |
| `report expression;` | Request host rendering of the result. The spelling adapts to established emission policy. |

Projection fields in these examples return scalars or positional arrays through
explicit domain projections. Native handles do not become constructed values.
`one` over a string projection returns a string; a count becomes an integer only
when it fits the shared signed range. Nested query results stay arrays unless
explicitly reduced or extracted. Projection over an empty selection returns an
empty positional array while still validating all field expressions.

The subject binder is explicit so that each field shows which item it uses.
It is a scoped language binding, not a user-authored callback or first-class
function. Earlier outer bindings retain their saved meanings. The entire body
is pure: edits and emitters are rejected before the projection starts.

The active request budget covers parameter admission, domain work, field
evaluation, generator candidates, reductions, copies and rendering. Validation
covers unexecuted branches and empty inputs. This document introduces no new
budget defaults or unit scheme; the existing brief and comprehension contract
remain the starting point.

## 1. A correlated per-function MLIR report

The question is: which functions have at least a caller-selected number of
operations, how many operations of a selected kind do they contain, and what
are their operation kinds?

### Input

```mlir
module {
  func.func @double(%x: i32) -> i32 {
    %0 = arith.addi %x, %x : i32
    func.return %0 : i32
  }
  func.func @mix(%x: i32, %y: i32) -> i32 {
    %0 = arith.muli %x, %y : i32
    %1 = arith.addi %0, %x : i32
    func.return %1 : i32
  }
  func.func @plain(%x: i32) -> i32 {
    func.return %x : i32
  }
}
```

Caller inputs are `operation_name = "arith.addi"` and `minimum_ops = 2`.
The report below counts body operations, including returns, and excludes the
function itself. These functions contain no nested operations, so
`children | subtree` has an unambiguous count.

### Complete proposed program

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
               | filter(op(operation_name)) | count,
  "kinds": f | children | subtree | names | unique
});
let selected = [r for r in rows if r.operations >= threshold];

report {
  "functions": selected,
  "total_operations": sum([r.operations for r in selected]),
  "total_matches": sum([r.matches for r in selected])
};
```

`op(operation_name)` passes a checked string to a registered domain predicate.
It does not manufacture another query string. `input` keeps Zirium's meaning
of all operations in the document. Each `f` starts a relative query from that
one function.

### Expected result

```json
{
  "functions": [
    {
      "name": "double",
      "operations": 2,
      "matches": 1,
      "kinds": ["arith.addi", "func.return"]
    },
    {
      "name": "mix",
      "operations": 3,
      "matches": 1,
      "kinds": ["arith.muli", "arith.addi", "func.return"]
    }
  ],
  "total_operations": 5,
  "total_matches": 2
}
```

This is the intended JSON data, not a promise about whitespace or object-key
rendering order. Native record labels preserve declaration order. Increasing
`minimum_ops` to 4 yields an empty `functions` array and zero totals. Setting
`operation_name` to an absent operation leaves both selected functions present
with zero matches.

Separate functions with the same name in different scopes would remain
separate rows. A missing or multiple-valued name fails at `one`; a duplicate
name across rows does not. No unique string key is required as it is for
`map_by`. A negative threshold fails before the native selection runs.

For grouping, the final report could instead consume
`group_by(rows, "matches")`. With this input it would return a group keyed by
integer 1 containing `double` and `mix`, followed by a group keyed by integer 0
containing `plain`. Groups follow first occurrence; members retain row order.
This is a follow-on use of shared grouping, not another report-specific stage.

### Ownership and discussion

Zirql owns parameters, expression-valued arguments, `project`, `one`, row
construction, comprehension filtering and reductions. Zirium supplies `op`,
`attr`, navigation and operation-name projection. Its adapter declares their
accepted inputs, results and purity; Zirql enforces composition and scope.

The explicit `project(f, {...})` form is the recommended starting point. The
shorter alternative is `project({...})` with implicit current input in each
field. Explicit subjects make correlation and nested projection easier to read;
implicit subjects make short reports less verbose. A native comprehension form
could also express this program, but would need a clear boundary between native
iteration and constructed scalar values.

## 2. A MiniZinc capacity report with value requirements

The question is: after scaling demand by a positive factor, is there enough
capacity at every corresponding position? Comparing only the totals gives the
wrong answer for this input.

### Input

```minizinc
demand = [2, 3, 4];
capacity = [6, 8, 5];
```

Caller input is `factor = 2`. These are literal integer arrays in a data file;
no model execution or solver is involved.

### Complete proposed program

```text
param factor: int;

source demand = literal("demand");
source capacity = literal("capacity");

let positive_factor = require(factor > 0, factor,
                              "factor must be positive");
let aligned_capacity = require(
  index_set(demand) = index_set(capacity),
  capacity,
  "demand and capacity must have the same index set"
);
let scaled = require(
  forall([d >= 0 | d in demand]) /\
  forall([c >= 0 | c in aligned_capacity]),
  [positive_factor * d | d in demand],
  "demand and capacity must be nonnegative"
);
let rows = [
  (index: i,
   demand: scaled[i],
   capacity: aligned_capacity[i],
   headroom: aligned_capacity[i] - scaled[i])
  | i in index_set(demand)
];
let total_demand = sum(scaled);
let total_capacity = sum(aligned_capacity);

report (
  rows: rows,
  total_demand: total_demand,
  total_capacity: total_capacity,
  totals_fit: total_demand <= total_capacity,
  cells_fit: forall([r.demand <= r.capacity | r in rows]),
  violations: [r | r in rows where r.demand > r.capacity]
);
```

`param` extends the program wrapper. `require`, `sum` and `forall` extend its
closed expression subset. The remaining record, array and Boolean notation
follows MiniZinc. These are shared language operations lowered from the CST,
not arbitrary model or standard-library calls.

### Expected native result

| Index | Scaled demand | Capacity | Headroom |
| --- | --- | --- | --- |
| 1 | 4 | 6 | 2 |
| 2 | 6 | 8 | 2 |
| 3 | 8 | 5 | -3 |

The result record has `total_demand = 18`, `total_capacity = 19`,
`totals_fit = true`, and `cells_fit = false`. `violations` contains the third
row only. The report is a successful analysis with a negative finding; it does
not fail evaluation merely because capacity is insufficient.

Zincite renders this native value through its existing tagged constructed-value
schema and `{complete, limitations, result}` envelope. The table is a readable
view of the value, not a proposed replacement wire format. For this literal-only
input, the expected completeness is true with no limitations.

### Requirements, empty input and keys

A zero or negative factor fails at its requirement. Both literal sources have
already been read at that point, following statement order. Unequal index sets
fail before cell access. A negative input cell fails before multiplication;
integer overflow during multiplication or summation fails the request. No
partial report reaches stdout after any such error.

For two empty positional integer arrays, totals are zero, both fit fields are
true and there are no rows or violations. If the application requires at least
one job, express that explicitly with `require(length(demand) > 0, ...)`.

This example deliberately uses positional arrays. Its ordinary comprehension
constructs fresh `1..n` positions. It must not be generalized unchanged to enum
or non-one-based keyed arrays: that would lose index identity. A keyed version
would construct `key: value` entries and retain explicit axes, especially for
empty results. Sparse multidimensional pairing additionally needs present-cell
keys, which motivates the research proposal for `entries` and exact alignment.

### Ownership and discussion

Zirql owns requirements, eager materialized-array reduction, checked arithmetic,
scope and generic index identity. Zincite owns `literal`, MiniZinc lowering,
nominal member resolution when needed, the report envelope and target policy.
This program constructs a report only. It does not settle or exercise the
attended data-edit contract.

Should a generator-call spelling such as `forall(i in index_set(demand))(...)`
be shorthand for materializing an array, or promise lazy head evaluation?
The recommendation is eager shorthand initially. A later lazy form should be
explicit, because skipped arithmetic failures and spent budget are observable.

`require` is a base special form with conditional evaluation, not an eager
ordinary function call. Advisory outcomes remain ordinary report data. A
future CI mode may turn `cells_fit = false` into a nonzero exit, but that is an
explicit host reporting policy rather than an arithmetic error.

## 3. Structural matches with correlated native captures

Using the MLIR input from example 1, find `arith.addi` operations whose operands
are the same SSA value. Keep the operation and both operand handles available
for further queries before rendering a scalar report.

### Complete proposed program

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

### Proposed matching rules

`from` enumerates a finite native selection. Each `bind` evaluates one value
for the current candidate, and `where` evaluates a Boolean guard. Clauses run
in written order. Later clauses can use earlier bindings; a name is not in
scope in its own source. Duplicate names and shadowing fail during validation.
Each complete match returns a native record containing `add`, `lhs`, and `rhs`.
Those names are local to a match and do not become top-level bindings.

`m.add` selects a field of that native record and preserves its operation type.
It is not a conversion to a constructed scalar. `project` then uses explicit
name projections to construct the final report. The native record may be
returned through a library API while its owning document remains alive.

`operand` is a proposed Zirium primitive returning one native SSA value.
Its operand indices are zero-based, matching the domain's operand numbering;
constructed positional arrays remain one-based. An unavailable operand is an
error in this required-binding form. Optional structural bindings need a
separately chosen meaning. `same_value` compares native value identity,
including result position or block-argument identity and owning document.

If another host can produce an Unknown guard, the recommendation for this
required-match form is a located refusal with the host's reason. Unknown must
not be silently treated as false. Partial semantic input also retains its
host completeness information through any known matches and their projections.
A form that deliberately returns partial matches would need an explicit
contract.

### Expected results and counterexamples

There is one native match: the addition in `double`, with `lhs` and `rhs` both
referring to that function's `%x` argument. The addition in `mix` uses `%0` and
`%x` and does not match.

```json
{
  "matches": [{"function": "double", "operation": "arith.addi"}],
  "count": 1
}
```

The comparison must not substitute `defs` identity for SSA identity. Two
different arguments can belong to the same function, and two different results
can come from the same multi-result operation. Neither pair is the same value.
Equal printed SSA names in different scopes also do not establish identity.

This is a read-only structural observation. It does not authorize replacing
`x + x`, establish rewrite legality, or add an algebraic simplifier.

### Ownership and discussion

Zirql owns `match`, native capture records, field access, scope, candidate
enumeration, guard evaluation and budget propagation. Zirium supplies the
operand and equality operations with typed signatures and purity declarations.
Zincite could use the same constructs with CST relations and proven declaration
identity. Each host retains its handles and source coordinates.

This resembles the existing comprehension generator chain. The implementation
should share its control rules where possible while retaining distinct native
and constructed value representations. Avoid requiring either host to implement
its own matcher control language.

The main choice is whether `match` deserves a separate surface form, or whether
native generators should be exposed through a more general comprehension form.
The explicit form above makes domain relations and captures readable. Native
comprehensions could reduce the number of constructs, but must make it clear
which outputs remain native and which undergo scalar construction. Snippet
patterns can later lower to the selected representation; they need not be the
first syntax.

## Decisions to discuss next

1. Use explicit subjects in `project`, or implicit current input? The examples
   recommend explicit subjects while preserving ordinary pipeline syntax.
2. Start with typed external parameters alone, or include declared result
   shapes? Parameters provide immediate reuse; result shapes help make reusable
   analysis interfaces reviewable before execution.
3. Expose native captures as `match`, or extend comprehension syntax to native
   domains? Either choice puts the execution and validation rules in Zirql.

These choices do not change the settled ownership direction. The base provides
the language; adapters supply domain knowledge and surface adaptations. No
tasks or implementation changes are implied by choosing an example spelling.
