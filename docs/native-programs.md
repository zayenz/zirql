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
representation. `parse_pipeline` reads only pipelines.
The [language direction examples](language-direction.md) show proposed surfaces.

Program expressions cover input, binding lookup, grouping, stage sequences,
ordered set operands and records. A record evaluates its fields in written order.
Set combination evaluates the left operand before the right; the host supplies
its domain identity and combination behavior.

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
