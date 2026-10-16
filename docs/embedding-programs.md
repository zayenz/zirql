# Construct programs in Rust

The word executable reads a single expression. A host that supports several
statements translates them into a `Program` and executes it through
`ProgramHost`. A caller can also build the same representation directly in Rust.
Use this path when your tool already has a parser or a typed query builder.

The examples below are adapter fragments. They assume the host types and context
described in [embedding in Rust](host-integration.md); they are not queries for
the word executable.

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

Check all caller parameters before reading domain data. `admit_parameters`
validates the supplied values and returns bindings ready for the program. This
example installs one integer parameter. The snippet belongs inside a function
returning `Result<_, zirql::QueryError>`:

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

Native hosts lift admitted `ConstructedValue` scalars into their result wrapper.
The example above uses initial bindings, as typed root builders do. For a text
program, retain the admitted values and lower each declaration to an ordinary
`Statement::Bind` at its written position. A host stage returns that
already-admitted scalar; the shared runner introduces its name into scope. An
earlier local scope can therefore use the same spelling as a parameter that is
not yet visible. The later declaration must still satisfy the program's
no-shadowing rule. Execute and render with the same `Budget`; repeated reads and
copies spend the remaining allowance.

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
expression, explicit bindings, the active budget and `FunctionLimits`. The
[compiled accumulator example in the API
source](https://github.com/zayenz/zirql/blob/main/src/functions.rs) shows the
full declaration construction. It counts down from 100 with an active function
depth of one. For the syntax it represents, see [function
declarations](functions-and-control-flow.md#declare-a-function).

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

`entry.kind()` supplies the host kind for subsequent program preflight. This
prepared unit borrows the immutable source trees and host. Keep it within this
submitted request, using the current host context and the same cumulative
budget. Prepared entries carry their unit identity; another unit refuses them.
Each execution releases its results and frames normally. A host can execute a
prepared projection field for each subject without preparing the function bodies
again. `Bindings::with_local` shares the outer map when introducing its subject;
native argument callbacks retain that sharing. Declaration inputs can be
borrowed iterators over separately shared typed-root function trees.
`FunctionRequest` retains the one-entry convenience API.

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
`tuple<T, ...>`, `record {field: T, ...}`, `set<T>`, `keyed<T>`, `queue<T>` and
`identity_set<T>`. Sets keep their supported Boolean, integer, float and nominal
element types. A bare saved name `fn` remains a name. The reader consumes a
declaration only after recognizing `fn name(`. Collect every declaration before
preparing the submitted program.

Bodies contain ordered `let name = expression;` bindings and a final expression.
Ordinary calls accept zero or more positional arguments. Conditionals use
`if condition { lets; expression } else { lets; expression }`, with optional
`else if` branches and a required final `else`. Every written branch receives
preflight checks; only the selected result executes. Branch-local names stay
inside their block. Ordinary JSON records remain value expressions.

Hosts with their own expression parser can implement
`ValueReaderHost::parse_function_value`. It receives the original cursor and
the function's current parameter/local names. Lower one initializer or final
expression and leave its `;` or closing `}` to the common declaration reader.
Returning `None` keeps the common expression grammar. The shared reader still
owns function headers, local binding order and declaration boundaries.

If parsing precedes model loading, use `read_unresolved_function_declaration`.
It retains located `SignatureType` names, including names nested in `set<T>`.
For each request, call `resolve_signature` with the current model's native and
nominal resolver. Keep the resulting small headers alive, pair each with its
parsed body using `with_signature`, then call
`PreparedFunctions::prepare_borrowed`. The resolved declarations borrow the
original bindings and final expression. They do not copy or reparse bodies. Use
owned declarations and `read_function_declaration` when parsing and type
resolution happen together. Use `dispose_unresolved_functions` when releasing
parsed native body trees.

Signature syntax, resolved headers and borrowed declaration slots spend the
active metadata allowance before growth. The common reader also pays for this
metadata when resolving an owned declaration immediately. Include that cost when
choosing the request's work and result allowances.

An immutable native host can opt into `FunctionHost::retains_native_admission`.
After the first whole-value check, an unchanged value passed under an equivalent
contract keeps its admission during that request. The host must guarantee stable
ownership, coordinates, contents and handle validity. The default continues to
check native values at every boundary. Fresh native results and changed contracts
still receive whole admission. This removes repeated native scans for opted-in
hosts; it does not increase the work/result defaults or skip producer charges.

Native matching inside a function uses `evaluate_match_with_arguments` to keep
the active runner while the common clause executor introduces captures. Its
source and guard hooks receive the current immutable clause bindings. A selected
embedded value runs through `NativeArguments::evaluate_match_value`; it does not
start another function request. `ProjectionHost::scoped_value_arguments` supplies
each written value's additional lexical names and optional relative input during
preparation. Earlier capture names are available to later sources and guards;
forward names and shadowing fail before execution. Ordinary arguments keep their
existing relative-input behavior and no additional names by default.

The same clause executor handles direct and function-bearing requests. It still
checks all written clauses, spends work on candidate/guard inspection, skips
later sources after a false guard and refuses a final Unknown. Captures share
their original outer binding map. Scoped preparation and the small clause-local
binding metadata spend the continuing request allowance before growth.

## Destructure values and select match arms

`Pattern` is a located `PatternKind`: a name, wildcard, fixed tuple, record,
scalar literal, `Present(child)` or `Absent`. A record contains located static
labels paired with child patterns. Use `ignore_rest: true` to ignore additional
stored fields. Renaming is the label/name pair; no rest value is allocated.

`ValueExpressionKind::Block { bindings, value }` contains ordered pattern/value
pairs and a final expression. Each initializer sees earlier bindings and the
outer scope. It evaluates once; all of its pattern must match before any of its
names become visible. Names disappear when the block finishes. Duplicate names
and shadowing fail during preparation, including in unused arms.

`PatternFunctionDeclaration` accepts `PatternParameter { pattern, declaration }`.
The declaration describes the whole argument. Use `evaluate_pattern_functions`
for constructed values, or `PreparedFunctions::prepare_patterns` and
`FunctionRequest::prepare_patterns` for a native host. These enter the same
runner as the existing name-only declarations. All arguments evaluate once,
left-to-right; all whole arguments are admitted before any parameter pattern
installs names. A mismatch points to its parameter pattern. Blocks hold local
pattern bindings in these declarations.

`ValueExpressionKind::ValueMatch { subject, arms }` uses ordered
`ValueMatchArm { pattern, guard, value }` entries. It evaluates the subject once.
A matching arm binds names only for its guard and value. A false guard continues;
a true guard selects that value. Guard errors, Unknown and non-Booleans fail.
Exactly one final unguarded `Wildcard` supplies the fallback. Only the selected
value executes. Selected final arms and block values preserve compatible tail
calls after pattern and guard work finishes.

Native records use `ProgramHost::capture_record` and
`ProjectionHost::capture_record_kind`. Native tuples can opt into `tuple_view`
and `tuple_view_kind`; their defaults expose no tuple. These views borrow an
immutable layout containing actual native children. They must remain stable
while the owner is retained. Hosts keep handle validity and source coordinates;
view access does not reflect Rust fields. Child selections retain the original
owner and any declared field/position contract. Host calls and returned owned
results use the existing charged materialization boundaries.

The common reader exposes these forms through `read_value_expression`,
`read_pattern`, `read_pattern_let` and `read_pattern_function_declaration`.
The pattern function reader accepts the complete parameter's type and lowers
local lets to the existing `Block`. Its name-only counterparts retain their
original signatures and representation. Names such as `if`, `match`, `let`,
`fn`, `param` and `report` remain contextual binding names. A bare `absent` or
`present` in a let or parameter position is a name; `present(pattern)` is an
explicit presence pattern, and nested/match `absent` is an absence pattern.

```zirql
fn add_cost((label, {cost: (_, cost), ..}):
    tuple<string, record {cost: tuple<string, int>, ignored: int}>) -> int {
    if cost >= 0 { let answer = cost + 1; answer }
    else { 0 }
}

match (lookup(row, "cost")) {
    absent => 0,
    present(cost) if cost >= 0 => { let answer = cost + 1; answer },
    _ => require(false, 0, "negative cost")
}
```

Tuple values use `(first, second)`; `(value)` remains grouping and `(value,)`
is a one-element tuple. Record results keep their JSON-shaped spelling,
including directly in a match arm. Function/conditional braces are blocks;
match arms also recognize blocks containing local lets and a final value.
Native `match { from ... }` retains its capture-enumeration grammar.
Host frontend availability is documented by each consumer.

For model-owned types, use `read_unresolved_pattern_function_declaration`, then
`resolve_signature` and `with_signature`. `ResolvedPatternFunctionSignature`
owns only request-resolved type metadata and borrows the original patterns.
`PreparedFunctions::prepare_patterns_borrowed` borrows that header and the
original body. Neither step clones or reparses the body or patterns.
`PreparedFunctions::value_kind` uses the same common preflight as preparation,
including lexical native field kinds through blocks and match arms. Hosts need
no separate block or pattern scope walk.

When kind discovery runs inside a native preparation callback, use
`pattern_function_value_kind` with borrowed pattern declarations. It checks the
signatures and the requested value through the same preflight, without preparing
function bodies again. Complete request preparation with
`PreparedFunctions::prepare_patterns` still checks every body before execution;
the signature query alone does not establish body purity. Frontend parsing can
use `pattern_names` to borrow ordered located names under its parse allowance,
without walking the pattern itself. The lexical owner checks those names against
the surrounding scope.

A top-level frontend can call `PreparedFunctions::prepare_binding` with the
pattern, initializer, outer kinds and optional relative kind. Preflight checks
the initializer's purity and all new names before execution. The returned
`PreparedPatternBinding::bindings` supplies ordered names and host kinds.
`evaluate_binding` evaluates once and returns a `NativeCapture` containing only
selected public outputs. It shares native/constructed owners while matching
and finishes selected fields through the ordinary host boundary; it does not
copy ignored fields into another record. Failed matching returns an error
before any caller binding changes.

Lower that capture to `Statement::BindFields { names, expression }` to install
its fields in an ordinary `Program`. Program preflight checks every new name
against the outer scope. Execution verifies and copies every requested field
through `capture_record` and `copy_value`, then installs the complete set.
`Statement` and `Program` retain their `Eq` support; the host retains its pattern
expression in its existing checked stage wrapper.

When releasing parsed native trees, use `dispose_native_value`,
`dispose_pattern_functions` or `dispose_unresolved_pattern_functions`;
`dispose_pattern` releases a standalone nested pattern iteratively. Reusable
typed wrappers can call `copy_pattern` to copy only located pattern metadata
under the active budget, without parsing generated text or copying matched data.

## Fold and scan named functions

Rust callers use `ValueExpressionKind::Fold { items, initial, step }` or `Scan`
with the same fields. `step` is a located function name in this closed position.
The [compiled fold example in the API
source](https://github.com/zayenz/zirql/blob/main/src/value.rs) adds `[2, 3]` to
initial state `0` with one active function frame. Replacing `Fold` with `Scan`
returns `[2, 5]`. The common reader parses `fold(items, initial, step)` and
`scan(items, initial, step)`, where `step` names a registered pure function.
The [function guide](functions-and-control-flow.md) explains the textual forms;
frontends still supply function registration and program handling.

The request resolves the step and checks its two-parameter signature and purity
before execution, including empty input. Its result declaration must be
compatible with parameter zero, which fixes the state contract. Optional
wrappers permit compatible signatures, but every actual state must satisfy that
fixed contract: an optional integer result cannot return absence to an integer
state. Initial state admission also runs for empty input.

Items evaluate first, then initial, once each. The eager positional input is
visited left-to-right, calling `step(state, item)` through the prepared function
runner. Keyed arrays need an explicit conversion. Fold returns initial on empty
input and the final state otherwise. Scan excludes initial and retains every
post-step state; empty scan retains its element declaration. Earlier snapshots
and aliases remain immutable.

Native hosts opt in through defaulted `FunctionHost::sequence_items` and
`construct_sequence` hooks. The first establishes positional storage and its
empty element contract, then enumerates original native handles in order. The
second assembles native scan snapshots, including typed empty output. Both
hooks preserve identity, source coordinates and aliases, and charge their scans
and allocations to the active budget. Existing hosts can leave these hooks
unsupported. Constructed states use ordinary constructed arrays.

## Repeat with explicit state

Rust frontends construct `ValueExpressionKind::Loop { states, body }`,
`Break(value)` and `Continue(values)`. These nodes use the same preparation and
runner as values and functions. The common reader parses `loop (name = initial,
...) { body }`, `break expression` and `continue (next, ...)`. Frontends connect
those expressions to their function and native-type handling, or construct the
same nodes directly.

Each state pairs a located fresh name with an initializer. All initializers
run left-to-right in the outer scope, before any state name becomes visible.
The body must end in a transfer on every path. A block's final expression,
conditional branches and value-match arms can contain those transfers. Ordinary
operands, guards and let initializers require values; they can contain a nested
loop, whose transfers belong to that inner loop. Transfers never cross a
function boundary. Even unused branches and function bodies receive these
checks during preparation.

The [compiled
accumulator](https://github.com/zayenz/zirql/blob/main/src/value.rs) adds 1
through 10,000 with an active function depth of one. `Continue` reads every
operand against the complete old state, checks every fixed state contract, then
installs the whole tuple. For example, a two-slot swap uses:

```rust
K::Continue(vec![name("right"), name("left")])
```

Here `name` constructs `ValueExpressionKind::Name`, as in the compiled example.
Both values see the old slots, so the two values exchange places. A separate
countdown slot can bound repeated swaps; all three replacement values must be
supplied.

Nested loops return ordinary values to the expression containing them. With the
same `node` and `int` helpers, this returns 7 from the inner loop, then breaks the
outer loop with that value:

```rust
let nested = node(K::Loop {
    states: vec![],
    body: Box::new(node(K::Break(Box::new(node(K::Loop {
        states: vec![],
        body: Box::new(node(K::Break(Box::new(int(7))))),
    }))))),
});
```

An inner `Continue` similarly advances only its own state. The Rust API checks
in
[`tests/value_expressions.rs`](https://github.com/zayenz/zirql/blob/main/tests/value_expressions.rs)
combine inner continues with outer block/match transfers, simultaneous swaps,
contract failures and ordinary work-limit refusal. A body containing only
`Continue` is valid; it eventually exhausts work rather than returning an
implicit value.

Initialization establishes each state's runtime contract. Function parameters
and results retain their explicit contracts; constructed annotations can supply
optional, empty collection or record schemas. Otherwise actual values establish
scalar, tuple, record, homogeneous collection or nominal contracts. Collection
length can change. Bare absence and collections without an element contract
cannot establish state by themselves. A native host's inferred kind must define
its whole native contract, including element information for empty native
collections; whole-value admission still checks the actual owner and coordinates.
A signature supplies that information when the runtime kind cannot.

State and iteration locals release on transfer, normal exit and failure.
Immutable outer aliases and selected native children keep their original owners.
A failed operand or admission never installs a partial next state. Work and
result spending remain cumulative; releasing state does not refund either.
