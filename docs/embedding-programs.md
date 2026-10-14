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


## Call typed pure functions

A `FunctionDeclaration` contains a located name, typed named parameters, a result
contract, ordered local bindings and one final value expression. Use
`ValueExpressionKind::Call { function, arguments }` at any ordinary expression
position. Declarations register together before bodies are prepared, so direct
and mutual recursion use the same path.

For constructed values, `evaluate_functions` takes declarations, an entry
expression, explicit bindings, the active budget and `FunctionLimits`.
The [compiled accumulator example in the API source](https://github.com/zayenz/zirql/blob/main/src/functions.rs)
shows the full declaration construction. It counts down from 100 with an active
function depth of one. These APIs are available through development Git; the
published alpha.5 does not include named functions.

For a native host, implement `FunctionHost` alongside `ProjectionHost` and
`RuntimeTypeHost`. A submitted program prepares its declarations once, then
resolves every written value in its lexical scope before executing statements:

```rust
let mut functions = PreparedFunctions::prepare(&declarations, &host, &mut budget)?;
let entry = functions.prepare_value(&value, &scope, Some(input_kind), &mut budget)?;
// Prepare the remaining written values here, before any operation executes.
let result = functions.evaluate(
    &entry, &bindings, Some(&input), &mut context, &mut budget,
    FunctionLimits { active_depth: 32 },
)?;
```

`entry.kind()` supplies the host kind for subsequent program preflight.
The unit borrows the immutable source trees and host. Keep it within this
submitted request, using the current host context and the same cumulative
budget. Prepared entries carry their unit identity; another unit refuses them.
Each execution releases its results and frames normally. A host can execute a
prepared projection field for each subject without preparing the function
bodies again. `FunctionRequest` retains the one-entry convenience API.

`result` is the host's `Value`. `NativeValue(query)` retains a pure native result;
`NativeQuery(query)` keeps its existing scalar-projection meaning. Native queries
inside functions read explicit parameters and earlier locals. There is no ambient
subject or captured program scope. A native stage can supply relative input to
its declared argument body; a function called there still receives its own inputs
explicitly. All arguments evaluate once in written order and pass whole-type
admission before any caller slots are replaced.

Final expressions and selected conditional, successful requirement and untyped
default branches preserve tail calls when their result declarations are exactly
equivalent. `1 + recurse(...)`, a typed default's pending fallback check, and an
incompatible result declaration retain a continuation and consume active depth.
The [limits guide](diagnostics-and-limits.md#function-calls-and-depth) describes
the independent guards and failure context.

The common reader accepts contextual declarations through
`read_function_declaration`. Its resolver handles registered native and nominal
names; Zirql reads `int`, `float`, `bool`, `string`, `opt T`, `sequence<T>`,
`tuple<T, ...>`, `record {field: T, ...}`, `set<T>` and `keyed<T>`. Sets keep their
supported Boolean, integer, float and nominal element types. A bare saved name
`fn` remains a name. The reader consumes a declaration only after recognizing
`fn name(`. Collect every declaration before preparing the submitted program.

Bodies contain ordered `let name = expression;` bindings and a final expression.
Ordinary calls accept zero or more positional arguments. Conditionals use
`if condition { expression } else { expression }`, with optional `else if`
branches and a required final `else`. Both written branches are preflighted;
only the selected result executes. Branch-local blocks belong to the later
patterns/control-flow work.
