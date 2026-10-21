# Explanatory findings with native subjects

Base-109, 2026-10-21. Small linters can return findings as an ordinary ordered
sequence of records. This design changes no runtime, reader, API or consumer code.

## Selected convention and authority

The user instructed: “Follow your recommendations and set zdev goal again.”
That delegates choosing the recommended minimal design. It does not approve
unseen syntax. Select ordinary records with `message: string`, `subject: native`
and optional `details` containing ordinary report data.

Each record is one finding. Preserve input order, separate findings and repeated
subjects. Two explanations about the same declaration or operation remain two
records. Any grouping, sorting or deduplication is an explicit caller choice.
The convention requires no severity field, rule registry or special finding type.
The host retains each subject's native identity and owner until rendering.

A hypothetical `finding(subject, message, details)` constructor would wrap the
same record. It adds no needed behavior to either example, so select no dedicated
helper. Ordinary native record construction already preserves the subject;
scalar projection is not the mechanism for retaining that native field.

## MiniZinc: an unused declaration

Reuse the complete model from Zincite's `docs/lint-scope.md`:

```minizinc
int: old_staff_limit = 12;
var 0 .. 10: staff;
constraint staff >= 4;
solve minimize staff;
```

The unused conclusion belongs to Zincite's rooted usage analysis, exposed by
`zincite_lint::resolve_unused_declarations`. Host policy requires
`ModelRootState::Complete` and `UsageOutcome::Unreachable`, with the existing
uncertainty and applicability exclusions. Constraints, solve, explicit/default
output and indirect dependencies contribute roots or reachability. Fragments
and uncertain reachable dependencies cannot establish this conclusion. Keep the
existing warning-source, suppression, underscore-name, declaration-role and
unused-enclosing-declaration exclusions when presenting this lint policy.

For that established conclusion, the host can construct this record shape:

```text
{"message": "Declaration is unused by this model",
 "subject": declaration,
 "details": {"name": "old_staff_limit"}}
```

Here `declaration` denotes the retained native `Declaration`. This is a
host-policy sketch, not runnable Zincite query syntax. Current query `uses`
describes written references; `uses | count == 0` cannot stand in for rooted
unused-declaration analysis. `UsageFacts` is not exposed as a query predicate by
this design. Removing the declaration remains a modelling decision.

## MLIR: review an integer conversion

The current selection `filter(op("arith.trunci"))` can select conversions for
review. Retain each selected operation as `subject`, attach the message
`Review integer narrowing conversion`, and put `operand_types` and `result_types`
in optional `details` as ordinary report data. For an `i64` to `i32` conversion,
those details describe `["i64"]` and `["i32"]`.

This is host policy based on the operation name and observed types. Selection is
a heuristic; it proves neither actual information loss nor dialect invalidity.
Zirium owns operation meaning and the native type observations. It can already
render these properties in an operation descriptor. The proposed details shape
does not introduce a query helper or claim every type property has query syntax.

## Chosen local demonstration

Adapt Zirium's `examples/cli/native-captures.zirql` against the existing
`examples/cli/function-report.mlir`. Keep its match prefix and replace the final
projection/report with an ordinary comprehension retaining the captured subject:

```zirql
let matches = match {
  from add in (input | filter(op("arith.addi")));
  bind lhs = operand(add, 0);
  bind rhs = operand(add, 1);
  where same_value(lhs, rhs);
};
[{"message": "Repeated SSA operand", "subject": m.add} for m in matches] | json
```

Run the saved adapted program from a Zirium checkout with
`zirium --program-file repeated-operands.zirql examples/cli/function-report.mlir`.
The adapted program and this expected output are source-checked, not executed
in this design task:

```json
[{"message":"Repeated SSA operand","subject":{"name":"arith.addi","attributes":{},"operand_types":["i32","i32"],"result_types":["i32"]}}]
```

The one result is the `arith.addi` in `double`; `mix` has distinct operands.
The message describes SSA operand identity, not a claim that the addition is a
bug. This demonstration exercises the record convention with current primitives.

## Rendering, uncertainty and failures

Zincite recursively renders native record leaves inside its semantic
`complete`/`limitations`/`result` envelope. Declaration leaves retain the actual
file, the declaration-name `range` and owning `syntax_range`. A constructed
record has no inherent input-source range; its native `subject` supplies it.
Existing host limitations remain attached to the result. Do not add a second
uncertainty protocol or turn missing facts into an unused-declaration finding.

Zirium recursively renders native record leaves. A singleton operation field
becomes the descriptor shown above, containing only `name`, `attributes`,
`operand_types` and `result_types`; that JSON carries no source coordinates.
Library callers can separately use `Document::operation_source_range` and the
optional `operation_location`. CLI `--jsonl` adds document input identity,
not an operation range. These boundaries promise no editor or SARIF integration.

Findings are successful result data. A failed `require`, invalid native handle,
unknown required observation or exhausted work/result budget remains a located
error. It must not become a finding or a truncated successful list. Existing
best-effort host output keeps its own incompleteness metadata and limitations.

## Documentation and bounded follow-ups

There are zero runtime follow-ups. Propose only these documentation slices:

- Zirql: explain the convention, native retention and error boundary in
  `docs/native-programs.md`, `docs/semantic-models.md` and embedding/API docs.
- Zincite, under host authority: a cookbook example separating rooted unused
  analysis from written uses, with native leaves and the semantic envelope.
- Zirium, under host authority: save the adapted example and document its actual
  descriptor output and separate library location access.

Defer query exposure of `UsageFacts` and located CLI finding rendering until a
concrete need selects their scope. No implementation tasks are imported here.
CP remains an optional ordinary-value boundary; this convention creates no
symbolic operation. Source review covered the host examples/guides, Zincite's
`program.rs`/`semantic.rs` and unused-declaration analysis, and Zirium's
`query/program.rs`, `query.rs` and `semantic.rs`. After applying these records,
the coordinator runs `git diff --check` and `zdev check base --format json`.
No Rust tests are needed for this design-only change.
