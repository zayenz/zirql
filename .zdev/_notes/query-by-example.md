# Query by example through existing patterns

## Selected scope

On 2026-10-21 the user said, “Follow your recommendations and set zdev goal
again.” The selected recommendation is to use existing record patterns in
selective generators as the first query-by-example experience. This delegates
the recommended minimum; it does not approve unseen native-template syntax.
This design changes no runtime, consumer frontend, dependency or public API.
Base-103 already implements `for case pattern in source` and the MiniZinc-shaped
`| case pattern in source`. Base-110's reference to an unimplemented selective
generator task is historical context. Reuse the current implementation.

A separate record-matching pipeline stage would repeat candidate selection,
pattern binding and rejection. It would also need rules for whether it returns
the original record, a capture record or a projected value. Selective generators
already make the source, captures and constructed result explicit. A bounded
native template could resemble copied host syntax more closely, but requires a
host parser and semantic rules before it can be a small, reviewable feature.
Select existing generators now; add no matching-stage sugar or template API.

## Record selection

The current [measurement example](../../docs/values.md#select-matching-values)
runs from a Zirql checkout through the existing CSV entry point:

```sh
cargo run --locked --example csv -- '
let records = [
  {"kind": "measurement", "name": "ash", "reading": 12},
  {"kind": "note", "name": "memo", "reading": 99},
  {"kind": "measurement", "name": "birch", "reading": null},
  5,
  {"kind": "measurement", "name": "oak", "reading": 7}
];
[{"name": name, "reading": reading}
 for case {kind: "measurement", name, reading: present(reading), ..} in records]
' examples/csv/people.csv
```

The documented result is `[{"name":"ash","reading":12},{"name":"oak","reading":7}]`.
Ash captures `name = "ash"`, `reading = 12`; oak captures `"oak"`, `7`.
Memo fails the literal `kind`, birch fails presence, and `5` fails record shape.
The equivalent generator spelling is
`[{"name": name, "reading": reading} | case {kind: "measurement", name, reading: present(reading), ..} in records]`.
These are supported core forms, source-reviewed here rather than newly executed.

Record labels are static; `{name}` means `{name: name}`. A final `..` admits
extra stored fields and neither captures nor copies them. Adding `unit: "C"`
to ash still matches the example; removing `..` makes that extra field a
mismatch. Every named field must exist in storage. A missing `reading` field
differs from a stored absent/null reading: both reject `present(reading)`, but
`{reading: absent, ..}` matches only the stored absent field. Use `lookup` to
treat missing-field access as optional data before matching.

Names become available together after the whole pattern succeeds. They remain
local to that candidate and cannot repeat or violate the scope's shadowing
rules. `{left: x, right: x}` is a duplicate-binding error, not equality testing.
For constructed equality, bind distinct names and write
`[left for case {left, right, ..} in pairs if left == right]`.
Literal patterns require compatible scalar kinds, including the same nominal
type/member identity; equal spelling or numeric magnitude does not coerce kinds.

Only structural, literal and presence mismatches skip the candidate, before its
guard, later sources and head. Ordinary generators and lets stay strict. Syntax,
duplicate names, invalid types/admission, host failures, Unknown guards and budget
exhaustion remain errors. Every written expression receives scope, type and
purity preflight even for empty sources or rejected candidates. Candidates still
spend work; existing source order, grouped expansion and result order remain.

## Copied MLIR and explanatory holes

Zirium's `examples/cli/function-report.mlir` contains this operation in `double`:

```mlir
%0 = arith.addi %x, %x : i32
```

Replacing its operands gives the explanatory sketch
`arith.addi ⟦operand⟧, ⟦operand⟧ : i32`. The result assignment is omitted because
this sketch concerns operand equality. The brackets are neither accepted query
syntax nor an approved template grammar. If such a template is designed later,
the repeated operand hole would require the same complete checked SSA identity.
It would not compare source text, type alone or the defining operation alone.
The `i32` text is fixture context, not a selected template type rule.

The existing native-capture language expresses that intent today:

```zirql
let matches = match {
  from add in (input | filter(op("arith.addi")));
  bind lhs = operand(add, 0);
  bind rhs = operand(add, 1);
  where same_value(lhs, rhs);
};
```

For `double`, one capture row retains `add` as that native addition and `lhs` and
`rhs` as the same native `%x` block argument. In `mix`, the copied operation
`%1 = arith.addi %0, %x : i32` deliberately does not match: `%0` and `%x` differ.
`same_value` includes the SSA result or block-argument position; two results of
one defining operation are different values. A known false guard rejects the
candidate. An absent, unresolved or invalid operand, a `bind` cardinality other
than one, or Unknown truth fails rather than silently producing a nonmatch.

The saved `examples/cli/native-captures.zirql` program projects these captures:
`zirium --program-file examples/cli/native-captures.zirql examples/cli/function-report.mlir`
Zirium reports `{"matches":[{"function":"double","operation":"arith.addi"}],"count":1}`.
The fixture, saved program and `docs/query-language.md` native-capture section
were source-reviewed here; this design task did not newly execute that command.

## Ownership and deferred options

`src/value/pattern.rs` owns structural/literal/presence matching over admitted
values and explicit native views. Shared comprehension preparation/execution
owns candidate scope, order, selection and work. `src/matching.rs` owns ordered
native captures, guard composition and shared limits. Hosts supply native
handles, owners, coordinates, views, operand extraction and checked identity.
There is no implicit native-field reflection or identity from rendered text.

Zirium owns MLIR parsing, dialect meaning and full SSA identity. Zincite retains
MiniZinc parsing, model facts, nominal identities, uncertainty and edit safety.
A future template must choose one host's concrete semantics before lowering to
these shared mechanisms. Do not introduce a generic AST interchange framework.
Defer template grammar, result/type/attribute holes, regions, variadics,
cross-operation graphs, alpha-equivalence, dialect equivalence, commutativity
and rewriting. None is necessary for the selected record experience.
CP remains a separate optional expression boundary returning ordinary values;
this choice creates no symbolic operation and has no CP merge dependency.

## Bounded follow-up and documentation

There is no required runtime implementation follow-up. At most, a Zirql
documentation-only slice may link the existing example from
`docs/functions-and-control-flow.md`, `docs/language-reference.md` and
`docs/embedding-programs.md`, retaining `ValueGenerator::In { case: true, .. }`
as the callable form. Existing links and examples already cover these paths;
avoid a duplicate demo or a separate syntax/API feature merely to name it.

Consumer adoption is separate work under each host's authority. Follow
[selective generator adoption](../../docs/host-integration.md#selective-generator-adoption):
adopt the dependency, keep existing builders strict with `case: false`, use
`case: true` for selection, and retain native views and admission checks.
Update that host's query reference and callable-builder documentation and run
its required checks before advertising support. Core/CSV availability alone
does not establish Zirium or Zincite frontend adoption. Zirium's current native
capture guide remains the supported alternative for the copied MLIR example.
This task imports no follow-ups and changes no runtime, consumer, publication or deployment. Completion needs the source/guide/fixture review above,
`git diff --check` and `zdev check base --format json`; no Rust tests are needed.
