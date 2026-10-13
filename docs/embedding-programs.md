# Construct programs in Rust

A frontend lowers its program syntax to `Program`. A caller can construct the
same located representation directly, then execute it through `ProgramHost`.
Use this path when your tool already has a parser or a typed query builder.

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

## Supply program operations

`ProgramHost` supplies the native value type, charged copying, stage execution,
set combination, record construction and emitters. Keep domain handles tied to
the context that owns them. Use `evaluate_expression` for an individual
expression, including a nested selector with relative input and the same budget.

The [host integration guide](host-integration.md) describes validation and
ownership requirements. The [program reference](native-programs.md) explains
scope, statement order and emission behavior.
