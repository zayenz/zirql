# Native programs

Use a program when several statements need the same native input or saved
results. Statements execute in written order. Bindings can save document
handles or borrowed selections for later use.

## Statements and scope

A binding statement evaluates an expression and installs an immutable name.
An evaluation statement evaluates an expression and can request implicit output.
Initial bindings are reserved for the whole program. Duplicate definitions and
forward references to unavailable names fail during preflight.

A host frontend chooses the statement spelling and lowers it to this
representation. `parse_pipeline` reads only pipelines. The common
`read_statement_prefix` recognizes `let name =`, `report` and
`param name: type` prefixes without reserving those words as bare binding names.
The host consumes the statement terminator after a parameter's type.
The [language direction examples](language-direction.md) show proposed surfaces.

Program expressions cover input, binding lookup, grouping, stage sequences,
ordered set operands, native capture field access and records. A record evaluates its fields in written order.
Set combination evaluates the left operand before the right; the host supplies
its domain identity and combination behavior.

## Required caller parameters

`ParameterDeclaration` records a located name and one of `ParameterType::Int`,
`String` or `Bool`. Declarations are metadata beside the host program. A text
frontend adds each name and its scalar type when it reaches that declaration;
an earlier statement cannot use a later parameter. Parameters follow the same
immutable, no-shadowing rules as saved values and local generator/projection names.
Defaults and undeclared caller inputs are unsupported.

Before domain execution, call `admit_parameters` with all declarations and either
`ParameterInputs::Typed` or `ParameterInputs::Cli`. It refuses duplicate
declarations, ambient-name collisions, duplicate supplied names, missing values,
extra names and incompatible types. It returns declaration-located scalar values
in declaration order. Lift those values into the host's native wrapper.
A text frontend lowers each parameter declaration to an ordinary `Statement::Bind`
at its written position; a host stage supplies the already-admitted value.
Admission checks all inputs up front, while ordered binding preserves scope during
execution. Typed root builders have no declaration statements: install their
parameters with admitted ambient values through `Bindings::new`.

The shared CLI decoder splits each `name=value` at the first `=`:

- `string` preserves the complete value, including empty text, quotes,
  backslashes, whitespace and further `=` characters.
- `int` accepts an optional `+` or `-`, then ASCII decimal digits fitting `i64`.
  Whitespace, prefixes such as `0x`, separators, decimals and overflow fail.
- `bool` accepts exactly `true` or `false`.

Neither input form substitutes text into the program. A host CLI can transport
repeated `--param name=value` arguments through the same decoder for inline and
file programs. Literal-only requests use empty declarations and inputs.
See [typed admission in Rust](embedding-programs.md#supply-typed-caller-inputs).

## Saved native values

A binding retains the host's value type. It can hold a borrowed selection,
a document handle or a constructed native record. Saving a value does not
render it. Nested selectors evaluate relative input in the same scope and
request budget.

The [Rust program construction guide](embedding-programs.md) shows how to
build and execute this representation without text syntax.

## Copying saved values

Reading a saved binding uses the host's `copy_value` hook. Earlier statements
also receive a charged copy of retained top-level input; the last receives the
original. A copy might duplicate a vector of handles without duplicating the
document. The host must admit produced entries and copied bytes before allocation.
The core admits record field containers and slots; moving already-admitted
children into them needs no second charge.

## Written and implicit output

A written emitter records either `EmissionPolicy::PassThrough` or
`EmissionPolicy::Terminal`. Pass-through allows later stages; terminal ends that
expression. An implicit emitter is requested only when the expression does not
end in a written emitter.

`ProgramHost::emit` receives the native value and the active budget. The host
owns rendering, snapshots, buffering and effects. Library emission does not
provide automatic rollback after a later error. A CLI that promises complete
output must buffer it until execution succeeds.

Pure bindings and nested constructor operands must be checked by the host
frontend before execution. The core cannot discover an emitter or edit hidden
inside an opaque domain payload. Read [host integration](host-integration.md)
for these validation obligations.

## Construct ordered report rows

A native stage can dispatch `project(subject, {fields})` to `evaluate_projection`.
The subject is visible only inside the fields and cannot shadow an outer name.
Native query fields use the existing expression representation; explicit scalar
projections produce constructed strings, integers or arrays. The result is a
positional array, so equal projected names require no unique key.

The core validates field scope, known types and effects before evaluating a row.
Nested selectors and value generators retain the same request budget. The host
keeps declared constructed schemas with saved bindings and renders at its
established emitter boundary. See
[embedding native projection](host-integration.md#correlated-native-projection)
for the callable adapter.

## Retain native captures

Use `NativeMatch<Q, P>` when a query needs correlated native handles before
constructing a scalar report. Its ordered, located clauses have three forms:

- `from name in query` enumerates finite subjects in their native order.
- `bind name = query` requires exactly one native subject for the candidate.
- `where predicate` keeps a candidate only when its final truth is known true.

Later clauses see earlier names. A name is absent in its own source; duplicate
names, ambient-name shadowing and the reserved input name fail preflight.
The rightmost `from` varies fastest. Capture names stay local to the match.
An unavailable required binding or an Unknown final guard gives a located error,
including when the guard is negated.

The common readers accept this surface for a host that supplies the native
primitives and match stage wrapper:

```text
match {
  from add in (input | filter(op("arith.addi")));
  bind lhs = operand(add, 0);
  bind rhs = operand(add, 1);
  where same_value(lhs, rhs);
}
```

`read_match` returns the clause tree. `read_native_expression` also reads a
match composed with later stages, such as a host's explicit `| json` emitter.
The word `match` is contextual before `{`, so a bare saved binding named
`match` remains readable. Inline and file frontends can use the same readers;
the core requires no file or entry-point declaration.

Each successful row is a host-wrapped `NativeCapture<Value>` with fields in
written order. Selecting `m.add` retains its native operation kind and uses a
charged native copy. A later `project(m, {"name": m.add | names | one})`
explicitly constructs a scalar report. Plain constructed-record field access
continues to use the scalar value evaluator.

`match_types` checks every clause, operand, Boolean branch, type and effect
before `evaluate_match` obtains subjects. Empty inputs and false guards do not
hide invalid clauses. Returned field kinds describe empty capture collections
too; the host retains that schema beside its collection wrapper. Matching
requires a program budget with aggregate result limits and charges candidates
before filtering. See [the adapter contract](host-integration.md#native-match-adapter).
These callable matching APIs require a development revision that contains them;
they are absent from the published alpha.5 release.

## Explain native connections

`NativePaths<S, P>` retains one typed edge, an ordinary target predicate and a
required nonnegative `max_depth` measured in edges. Supporting frontends use
`read_paths` for the common form:

```text
paths(edge, to=predicate, max_depth=2)
```

The edge is one registered identifier. It cannot be a pipeline. The target
uses the shared `not`, `and`, `or` and grouping rules. Typed callers build the
same representation with located edge, target and bound values.

`evaluate_paths` returns one shortest witness for each reachable source/sink
pair within the bound. Distinct sources keep their incoming identity order.
For each source, sinks follow breadth-first discovery order, and the host's
neighbor order chooses between equal-length witnesses. Repeated source handles
and cycles do not duplicate a pair. Searching continues through matching sinks.

Each row is a native capture with ordered `source`, `sink` and `steps` fields.
Endpoints remain singleton native selections. Steps retain an ordered native
selection containing both endpoints, so a matching source yields `[source]`
at depth zero. Field access and `project` use the existing native capture path;
no handle becomes a scalar or JSON value during traversal.

The evaluator preflights the edge and every target branch before obtaining
subjects, including empty input and unchosen Boolean branches. A final Unknown
target refuses at its query range; host failures retain their responsible
source locations. Separate host limitations remain attached to the request.
No witness within the bound does not establish global disconnection.

Paths require aggregate program limits and share the active budget through
scans, identity comparisons, traversal slots, witness copies and rendering.
See [the native path adapter](host-integration.md#native-path-adapter).
These callable APIs require a development revision; they are absent from
published alpha.5. Consumer adoption is separate: a host must supply its edge
adapter and stage wrapper before its CLI or library accepts paths.
