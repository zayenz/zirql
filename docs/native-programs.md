# Native programs

Use a program when several statements need the same native input or saved
results. `Program` retains statement order; `ProgramHost` supplies operations
on the host's value type. Bindings can save document handles or borrowed
selections for later use.

## Statements and scope

`Statement::Bind` evaluates an expression and installs an immutable binding.
`Statement::Evaluate` evaluates an expression and can request implicit output.
Initial bindings are reserved for the whole program. Duplicate definitions and
forward references to unavailable names fail during preflight.

A host frontend chooses the statement spelling and lowers it to this
representation. `parse_pipeline` reads only pipelines.
The [language direction examples](language-direction.md) show proposed surfaces.

`ExpressionKind` covers input, binding lookup, grouping, stage sequences, ordered
set operands and records. A record evaluates its fields in written order.
Set combination evaluates the left operand before the right; the host supplies
its domain identity and combination behavior.

## A native binding in Rust

This constructs a program that saves its initial input under `selected`:

```rust
use zirql::{ExpressionKind, Located, Program, Statement};

let program: Program<(), (), ()> = Program::new(
    vec![Located::new(
        Statement::Bind {
            name: Located::new("selected".into(), 4..12),
            expression: Located::new(ExpressionKind::Input, 15..20),
        },
        0..21,
    )],
    0..21,
);
```

Constructing the program runs no host hooks. To execute it, call
`evaluate_program` with a `ProgramHost`, its native input, mutable context,
initial `Bindings` and the active `Budget`. The returned bindings retain the
host's value type. This example uses unit payload types because there are no
stages, set operators or emitters in it.

`evaluate_expression` is the corresponding entry point for one expression.
A nested host selector can call it with relative input while retaining the
same context and budget.

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
