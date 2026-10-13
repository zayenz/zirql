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

## Supply typed caller inputs

Admit the entire input set before reading domain data. This example installs
one integer parameter through the existing immutable binding path. The snippet
belongs inside a function returning `Result<_, zirql::QueryError>`:

```rust
use zirql::{
    Bindings, Budget, ConstructedValue, Located, ParameterDeclaration,
    ParameterInputs, ParameterType, ProgramLimits, admit_parameters,
};

let declarations = [ParameterDeclaration {
    name: Located::new("factor".into(), 6..12),
    ty: Located::new(ParameterType::Int, 14..17),
}];
let inputs = [("factor".into(), ConstructedValue::Integer(2))];
let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000,
    results: 1_000,
    nesting: 16,
    stream_items: 100,
});
let admitted = admit_parameters(
    &declarations, ParameterInputs::Typed(&inputs), &[], 0..18, &mut budget,
)?;
let bindings = Bindings::new(admitted)?;
assert_eq!(bindings.get("factor", 30..36)?, &ConstructedValue::Integer(2));
```

For a CLI request, pass its borrowed raw argument list as `ParameterInputs::Cli`.
The [parameter reference](native-programs.md#required-caller-parameters) defines
the exact decoding rules. Hosts reserve ambient names separately; for example,
`&["document"]` prevents a parameter from replacing an ambient document binding.
Calls with no inputs pass `ParameterInputs::Typed(&[])` and still reject a
program's missing required parameters.

Convert each declaration's type to a scalar fact with `ValueType::from` for
expression and primitive-argument preflight. Text frontends retain written
declaration scope; admitting all inputs does not make a later declaration visible
to earlier statements. When adapting a typed root builder, retain its declarations
through erasure, appended stages and composition. Validate that metadata on every request.

Real hosts lift admitted `ConstructedValue` scalars into their existing native
result wrapper. The example above uses initial bindings, as typed root builders do.
For a text program, retain the admitted values and lower each declaration to an
ordinary `Statement::Bind` at its written position. A host stage returns that
already-admitted scalar; the shared runner introduces its name into scope.
This lets an earlier local binder use a name declared as a parameter later.
Execute and render with the same `Budget`; repeated reads and copies spend the
remaining allowance.

## Supply program operations

`ProgramHost` supplies the native value type, charged copying, stage execution,
set combination, record construction and emitters. Keep domain handles tied to
the context that owns them. Use `evaluate_expression` for an individual
expression, including a nested selector with relative input and the same budget.

The [host integration guide](host-integration.md) describes validation and
ownership requirements. The [program reference](native-programs.md) explains
scope, statement order and emission behavior.
