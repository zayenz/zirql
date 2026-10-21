# Construct programs in Rust

For constructed data, bind an admitted value and declaration with
`ConstructedBinding`, then read and evaluate through `ConstructedProgram`.
For native handles and domain operations, build or read the located core
`Statement` forms and prepare them with `PreparedProgram` and your native host.
Both routes use the same common reader, scope rules, functions and evaluator.
A typed query builder can construct the core representation directly in Rust.

## Constructed data programs

This minimal example needs no native host traits. The producer owns input
allocation; the continuing budget covers admission and the submitted program:

```rust
use zirql::{Budget, ConstructedBinding, ConstructedProgram, ConstructedValue as V,
            DeclaredType as D, FunctionLimits, Location, ParameterInputs, ProgramLimits};
let mut budget = Budget::for_program(ProgramLimits {
    work: 100_000, results: 10_000, nesting: 64, stream_items: 100,
});
let numbers = ConstructedBinding::new(
    V::Array(vec![V::Integer(3), V::Integer(5)]),
    D::Sequence(Box::new(D::Integer)), &mut budget, Location::Source(0..0),
)?;
let program = ConstructedProgram::read("numbers | count", &["numbers"], &mut budget, 64)?;
let result = program.evaluate(
    &[("numbers", &numbers)], Some(&numbers), ParameterInputs::Cli(&[]),
    FunctionLimits { active_depth: 16 }, &mut budget,
)?;
assert_eq!(result, V::Integer(2));
```

A `ConstructedBinding` stores its admitted value and compact declaration facts.
Use `Sequence` for positional data; empty values retain their element schema.
Reuse the same borrowed binding under several names or as current input.
`ConstructedProgram::read` accepts functions, typed parameters, ordered pattern
lets and one final value expression. `evaluate` admits `ParameterInputs` and
prepares the common program before execution. Functions receive all values
explicitly and cannot capture ambient bindings, parameters or current input.
Failures return located `FunctionError<QueryError>` values with bounded call
frames. Render a completed result explicitly with the shared
[JSON or Markdown renderer](library-rendering.md).

This API is newer than registry Zirql 0.2.0. Use the working checkout described
in [setup](getting-started.md#use-zirql-in-another-rust-project).
[Build the CSV host](csv-host.md) follows file loading, declaration, binding,
program execution and rendering. The
[parameterized orders report](csv.md#calculate-with-numeric-cells) demonstrates
named functions and repeated CLI parameter values.

The remaining examples are native adapter fragments. They assume the host types
and context described in [embedding in Rust](host-integration.md).

## Read text programs

`ProgramReaderHost` extends `ValueReaderHost` with the surface decisions needed
by native programs. Call `read_program(source, initial_names, host, budget,
nesting)` with original source text, the names supplied by the caller and the
same budget used for preparation and execution. The result contains unresolved
pattern functions, ordered parameters with their statement positions, located
statements and the original program range. Reading runs no native operations.

The reader first collects top-level function headers, skipping strings, comments
and balanced bodies. It exposes all function names before reading any body.
Function parameters and local lets form each body's scope; program parameters,
bindings and current input are unavailable there. The reader then handles
`param`, pattern `let`, `report`, separators and evaluations in written order.
A parameter or binding becomes visible only after its declaration. Duplicate
names retain the range of the repeated definition.

The host's `read_program_expression` callback consumes one expression and
receives its purpose, current lexical names, budget and remaining nesting
allowance. The driver borrows initial and lexical names, admits its source scan
and retained metadata against that budget, and releases partial trees on failure.
A host admits its own lexer and native syntax storage without charging the common
source scan again. Other callbacks admit native signatures and binding patterns, lower
parameters and patterns to core statements, recognize domain statements or
legacy headers, and select implicit output. They do not maintain another common
statement loop. Zincite's callback retains its atomic edit grammar; Zirium's
retains `do`, legacy bindings and pass-through emission.

For a constructed data application, use `ConstructedProgram` above to connect
these steps. The lower-level `read_value_program` entry remains available for
adapters that deliberately need only pattern lets and a final value expression;
it does not admit functions, parameters or `report` statements.

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

## Build core value and projection steps

In the development API, ordinary values and correlated projections are core
steps inside `ExpressionKind::Sequence`. A typed caller can construct them
without a host stage wrapper:

```rust
use zirql::{
    ConstructedValue, Expression, ExpressionKind, ExpressionStep, Located,
    Projection, ValueExpressionKind,
};

let value = Located::new(ValueExpressionKind::Literal(ConstructedValue::Integer(1)), 0..1);
let expression: Expression<(), (), ()> = Located::new(ExpressionKind::Sequence {
    input: Box::new(Located::new(ExpressionKind::Input, 0..1)),
    steps: vec![Located::new(ExpressionStep::Value(value.into()), 0..1)],
}, 0..1);
let projection: ExpressionStep<(), (), ()> = ExpressionStep::Projection(Projection {
    subject: Located::new("row".into(), 8..11),
    fields: vec![(Located::new("n".into(), 14..17),
        Located::new(ValueExpressionKind::Literal(ConstructedValue::Integer(1)), 19..20))],
}.into());
```

The snippet constructs syntax only. The [native projection example](native-programs.md#project-a-native-selection)
runs a projection through Zirium. To execute core steps, prepare the expression
through the request's `PreparedProgram` as below. `Stage` still carries
domain operations and `Emit` carries explicit host output. Registry Zirql 0.2.0
predates these core steps.

## Build native composition steps

Filters, matches, ordering, maps, existence, fixed points and paths are direct
`ExpressionStep` forms. A typed builder retains a nested native expression in
`NativeBody` rather than adding an executable host wrapper:

```rust
use zirql::{Expression, ExpressionKind, ExpressionStep, Located, NativeBody};

let relation: Expression<(), (), ()> = Located::new(ExpressionKind::Input, 13..18);
let exists: ExpressionStep<(), (), ()> =
    ExpressionStep::WhereExists(NativeBody::new(relation, 13..18).into());
let selection: Expression<(), (), ()> = Located::new(ExpressionKind::Input, 9..14);
let fixed_point: ExpressionStep<(), (), ()> =
    ExpressionStep::FixedPoint(NativeBody::new(selection, 9..14).into());
```

These fragments construct syntax. The host must admit the selection kinds and
supply native subjects and shape restoration before execution. The common
reader constructs the same forms from `where_exists(input)` and
`fixpoint(input)`. Existence runs its body against each singleton; a fixed-point
body receives the whole current selection.

A native-only adapter implements `ProjectionHost` and calls
`evaluate_native_expression` with current input, context, bindings and budget.
Use `native_entry_kind` for entry-level type checking that admits the host's
ordinary effects; `native_expression_kind` checks pure nested expressions.
Function-bearing requests prepare the same tree through `PreparedFunctions`
and execute it with the active invocation. `NativeArguments::evaluate_body`
retains a body's native result; ordinary value expressions keep their explicit
scalar projection rules. Hosts do not need function capabilities for native-only
composition.

Host callbacks describe native kinds, guard leaves, identity, scalar keys,
selection or dictionary shape and domain admission. The core owns common
argument discovery, scopes, purity checks and execution. See the
[adapter hooks](host-integration.md#native-ordering-and-map-construction).
The [Zirium projection recipe](native-programs.md#project-a-native-selection)
exercises this boundary with real native operations.

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
program, retain the admitted values and use `Statement::Parameter` at each
written declaration. Pass the admitted values to `PreparedProgram::evaluate`;
the shared statement driver moves each scalar into scope at that position. An
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
source](https://github.com/zayenz/zirql/blob/v0.2.0/src/functions.rs) shows the
full declaration construction. It counts down from 100 with an active function
depth of one. For the syntax it represents, see [function
declarations](functions-and-control-flow.md#declare-a-function).

For a native host, implement `FunctionHost` alongside `ProjectionHost` and
`RuntimeTypeHost`. Read a native expression with `read_native_expression`, or
build the same located tree in Rust. A submitted program prepares its declarations
once, then resolves every written expression in its lexical scope before
executing statements:

```rust
let functions = PreparedFunctions::prepare(&declarations, &host, &mut budget)?;
let prepared = PreparedProgram::prepare(
    functions, &program, &scope, Some(input_kind),
    FunctionLimits { active_depth: 32 }, &mut budget,
)?;
let bindings = prepared.evaluate(
    bindings, admitted_parameters, Some(input), &mut context, &mut budget,
)?;
```

`PreparedFunctions` owns callable declarations and checks every function body,
including unused ones. `PreparedProgram` borrows the canonical lowered `Program`,
retains its prepared entries and advances binding kinds in statement order.
`Statement::BindPattern` retains a pattern and pure value initializer directly;
its selected fields enter scope together after matching succeeds. Native capture
selection can still use `BindFields` independently.

Keep the prepared owner within the submitted request. Mutable context, results
and the active budget remain outside it. The shared statement driver handles
input copies, binding installation and output order for direct and prepared
entry points. Hosts retain native admission, document ownership and emitters.

The compiled control
`prepared_program_combines_functions_parameters_patterns_and_current_input` in
`tests/native_programs.rs` uses the existing borrowed-source host. It prepares
`increment(value: int)`, admits `factor = 2`, destructures `n = 3` from constructed
current input, reads the initial shared binding `shared = 4`, and saves
`increment(n + factor + shared) = 10`. Admission, preparation and execution
use one budget. Run the complete example with:

```sh
cargo test --locked --test native_programs prepared_program_combines_functions_parameters_patterns_and_current_input
```

`scope` maps visible names to the host's `ProjectionHost::Kind`. Pass `None` for
the relative kind and input when the expression has no current input. A core `Value`
step uses this actual optional input; its surrounding `ExpressionKind::Input`
wrapper does not grant a function access to an ambient subject.

A core `Projection` step uses this same preparation call. Zirql obtains the
declared singleton kind, checks every field and retains its record alternatives
before obtaining subjects. Execution runs fields in written order through the
active function argument runner, including for nested projections. Hosts supply
subject and scalar views and preserve the prepared types when lifting the
result, so empty reports retain their schema. The existing
`tests/native_programs.rs` controls exercise this sequence in
`native_captures_keep_written_fields_and_feed_shared_projection` and nested
function projections.

Use `evaluate_program_with` to run statements through prepared entries while
sharing the direct native program's sequencing, input copies and emission policy.
The host retains declaration and statement bookkeeping around those entries.

`Bindings::with_local` shares the outer map when introducing a local name;
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
names; Zirql reads `int`, `float`, `bool`, `string`, `value`, `opt T`,
`sequence<T>`, `tuple<T, ...>`, `record {field: T, ...}`, `dict<K, V>`, `set<T>`,
`keyed<T>`, `queue<T>` and `identity_set<T>`. `value` accepts any constructed
value, including collection keys and elements through `dict<value, value>`
and `set<value>`. Scalar and nominal set contracts remain strict. A bare saved name `fn` remains a name. The reader consumes a
declaration only after recognizing `fn name(`. Collect every declaration before
preparing the submitted program.

Bodies contain ordered `let name = expression;` bindings and a final expression.
Ordinary calls accept zero or more positional arguments. Conditionals use
`if condition { lets; expression } else { lets; expression }`, with optional
`else if` branches and a required final `else`. Every written branch receives
preflight checks; only the selected result executes. Branch-local names stay
inside their block. Ordinary JSON records remain value expressions.

Function initializers and results use the shared expression reader. Both
`if ... { ... } else { ... }` and `if ... then ... elseif ... else ... endif`
lower to the same conditional nodes. Native operation arguments call
`read_value_expression_in_scope` with their borrowed lexical names, the same
budget and the remaining allowance received by their scoped reader hook.
Nested reader entry cannot widen the enclosing request allowance. A recognized domain signature can use
`ValueReaderHost::parse_domain_call`; see [domain calls](host-integration.md#shared-query-grammar-and-domain-calls).
The shared reader owns generic grammar and declaration boundaries. The host
admits every written branch and retains its native and effect rules.

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

Native matching inside a function uses the core `Match` step. The common
clause executor introduces captures and retains the active runner; sources and
guards see the original query input plus only earlier captures. Forward names
and shadowing fail before execution. Only opaque native leaf arguments need
`ProjectionHost::scoped_value_arguments`; hosts do not walk common match clauses.

The same executor checks all written clauses, spends work on candidates and
guards, skips later sources after a false guard and refuses a final Unknown.
Captures share their original outer binding map. Scoped preparation and the
clause-local binding metadata spend the continuing request allowance.

## Define a native function

An eager pure host function uses the same `Call` node as a declared function.
For example, inside `FunctionHost<MyContext> for MyHost`, a native identity
function can retain one `Node` without copying its handle:

```rust
fn native_function(
    &self, name: &str, budget: &mut Budget, range: &Range<usize>,
) -> Result<Option<NativeFunction<MyContext, Self>>, QueryError> {
    if name != "keep_node" { return Ok(None); }
    let location = Location::Query(range.clone());
    budget.charge(2 * std::mem::size_of::<DeclaredType<MyKind>>(), location.clone())?;
    budget.charge_results(2, location)?;
    Ok(Some(NativeFunction {
        name: "keep_node",
        signature: OperationSignature {
            parameters: vec![DeclaredType::Native(MyKind::Node)],
            result: DeclaredType::Native(MyKind::Node),
        },
        invoke: |_, mut arguments, _, _, _| Ok(arguments.pop().unwrap().value),
    }))
}
```

`MyKind::Node` is the host's singleton kind. Its `admit_native` implementation
checks the actual owner, coordinates and singleton representation. Larger
signatures use `copy_declaration` for charged copies of existing declarations.
The compiled `NativeFunction` API example in `src/functions.rs` shows that
pattern without requiring a concrete host.

Read, prepare and evaluate with the existing request APIs:

```rust
let mut reader = ArgumentReader::new("keep_node(selected)", 0)?;
let value = read_value_expression_in_scope(
    &mut reader, &mut reader_host, &["selected"], &mut budget, 64,
)?;
let mut functions = PreparedFunctions::prepare([], &host, &mut budget)?;
let entry = functions.prepare_value(&value, &scope, None, &mut budget)?;
let result = functions.evaluate(
    &entry, &bindings, None, &mut context, &mut budget,
    FunctionLimits { active_depth: 32 },
)?;
```

`scope` declares `selected`'s kind and `bindings` holds its actual value. The
reader needs no function registry. Preparation retains the resolved definition,
checks every written argument, and derives the result kind from its declaration.
Execution passes admitted arguments with their written ranges to the native
implementation. `checked_invoke` admits its result before it returns to the
ordinary runner. Native and empty sequence results keep their declared facts;
rendering remains a later explicit step. Native functions cannot be named
`fold` or `scan` steps; those steps require a declared function body.

Zincite's [recursive call counter](https://github.com/zayenz/zincite/blob/main/docs/queries.md#count-written-calls-with-a-function)
and Zirium's [subtree cost program](https://github.com/zayenz/zirium/blob/main/examples/cli/subtree-cost.zirql)
use this path with domain functions defined by their hosts.

## Destructure values and select match arms

`Pattern` is a located `PatternKind`: a name, wildcard, fixed tuple, positional
list, record, scalar literal, `Present(child)` or `Absent`. A record contains located static
labels paired with child patterns. Use `ignore_rest: true` to ignore additional
stored fields. `List { elements, ignore_rest }` requires exact list length unless
`ignore_rest` discards a final remainder. It never binds or copies that remainder.
Renaming is the label/name pair; no rest value is allocated.

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
and `tuple_view_kind`; their defaults expose no tuple. Positional lists use
`positional_view` and `positional_view_kind`, with the element kind retained for
empty input. Ordinary predicate helpers can inspect constructed wrappers through
`ProjectionHost::pattern_constructed`. These views borrow an
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
`fn`, `param` and `report` remain contextual binding names. In every pattern
position, `absent` tests absence and `present(pattern)` tests presence. A bare
`present` remains an ordinary name.

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
The host retains its pattern expression in its existing checked stage wrapper.

When releasing parsed native trees, use `dispose_native_value`,
`dispose_pattern_functions` or `dispose_unresolved_pattern_functions`;
`dispose_pattern` releases a standalone nested pattern iteratively. Reusable
typed wrappers can call `copy_pattern` to copy only located pattern metadata
under the active budget, without parsing generated text or copying matched data.

## Fold and scan named functions

Rust callers use `ValueExpressionKind::Fold { items, initial, step }` or `Scan`
with the same fields. `step` is a located function name in this closed position.
The [compiled fold example in the API
source](https://github.com/zayenz/zirql/blob/v0.2.0/src/value.rs) adds `[2, 3]` to
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
accumulator](https://github.com/zayenz/zirql/blob/v0.2.0/src/value.rs) adds 1
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
[`tests/value_expressions.rs`](https://github.com/zayenz/zirql/blob/v0.2.0/tests/value_expressions.rs)
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

## Scoped mapping in Rust

`ValueExpressionKind::Map` and `FlatMap` take an `array` expression and an
`argument: PredicateArgument<Q>`. The argument's optional pattern matches each
item strictly. Its body is an ordinary value expression; the enclosing stage
sets its result contract. `Map` accepts any supported value, while `FlatMap`
requires positional sequence results. `PredicateArgument` is a scoped body,
not a first-class function value.

This callable example maps two integers without a text frontend:

```rust
use zirql::{
    Bindings, Budget, ConstructedValue as V, Located, PredicateArgument,
    ProgramLimits, ValueExpression, ValueExpressionKind as K, evaluate_value,
};

let expression: ValueExpression = Located::new(K::Map {
    array: Box::new(Located::new(K::Literal(V::Array(vec![
        V::Integer(2), V::Integer(3),
    ])), 0..6)),
    argument: PredicateArgument {
        argument: None,
        body: Box::new(Located::new(K::PredicateSubject, 13..14)),
    },
}, 0..15);
let result = evaluate_value(&expression, &Bindings::default(),
    &mut Budget::for_program(ProgramLimits {
        work: 10_000, results: 1_000, nesting: 16, stream_items: 100,
    })).unwrap();
assert_eq!(result, V::Array(vec![V::Integer(2), V::Integer(3)]));
```

Use `ValueSuffix::Map` or `FlatMap` with `lower_value_suffix` when a host already
has a source value tree. `read_value_suffix` reads their common textual forms,
including pattern headers and lexical captures. Preserve the supplied ranges
and remaining nesting allowance. Bare `map(...)` and `flat_map(...)` use the
constructed current-input hooks.

Native function requests require `ProjectionHost::positional_view_kind` and
`ProgramHost::positional_view` to agree on positional sequence storage and its
element kind. Opaque native selections have no implicit positional meaning.
`FunctionHost::construct_list` and `ProgramHost::construct_record` retain native
children in mapped reports. Bodies run once per item through the shared subject
scope and comprehension runner; nested scopes rebind `_` while explicit outer
names retain their handles and source coordinates. Saved reports retain the
kinds needed by later native expressions. Every body receives preflight even
when its input is empty. Iteration, intermediate construction, copies and output
append operations share one budget; stream limits bound the expanded result.
A failed request returns no partial report.

## Consumer adoption

The word and CSV examples expose these stages through the common value reader.
Zincite and Zirium need frontend routing to the shared `Map`/`FlatMap` nodes
before their query surfaces can advertise the same syntax. Their adapters must
supply positional type/value views and native construction hooks for their own
sequences and report values. Domain projections remain explicit.

Zincite retains MiniZinc types, model facts and its Data expression/edit policy.
A capacity report like the local example needs the host's capacity and demand
projections before it can read model-backed values. Zirium retains MLIR kinds,
handle identity and source coordinates. Each consumer also owns its rendering
and JSON policy for retained handles. The ordinary mapping operations do not
extend a symbolic expression subset.

## Select matching generator candidates

`ValueGenerator::In::case` chooses whether a pattern mismatch skips the candidate.
Set it to `false` for ordinary strict generators and to `true` for the textual
`for case` or `| case` forms. `ValueGenerator::Bind` remains strict. This callable
example selects present integers:

```rust
use zirql::{
    Bindings, Budget, ConstructedValue as V, Located, PatternKind as P,
    ProgramLimits, ValueConstructor, ValueExpression, ValueExpressionKind as K,
    ValueGenerator, evaluate_value,
};

let expression: ValueExpression = Located::new(K::Comprehension {
    constructor: ValueConstructor::Array,
    clauses: vec![Located::new(ValueGenerator::In {
        case: true,
        binders: vec![Located::new(P::Present(Box::new(Located::new(
            P::Name(Located::new("n".into(), 18..19)), 18..19,
        ))), 10..20)],
        source: Located::new(K::Literal(V::Array(vec![
            V::Integer(1), V::Absent, V::Integer(2),
        ])), 24..36),
        filter: None,
    }, 3..36)],
    head: Box::new(Located::new(K::Name("n".into()), 1..2)),
}, 0..37);
let result = evaluate_value(&expression, &Bindings::default(),
    &mut Budget::for_program(ProgramLimits {
        work: 10_000, results: 1_000, nesting: 16, stream_items: 100,
    })).unwrap();
assert_eq!(result, V::Array(vec![V::Integer(1), V::Integer(2)]));
```

The shared matcher retains native handles through the host's existing pattern
views. Successful names are local to the candidate; mismatches skip its guard,
later sources and head. Host/admission errors, runtime failures and Unknown
guards propagate. Preflight still checks every written expression, and rejected
candidates share the request's work limits. The [CSV example](values.md#select-matching-values)
shows the same selection over mixed measurement records. [Consumer adoption](host-integration.md#selective-generator-adoption)
requires the corresponding dependency and builder changes.

## Windows and consecutive runs

Build `ValueExpressionKind::Windows { items, size }` with two boxed expressions,
or `ValueExpressionKind::RunsBy { rows, field }` with a boxed source and a
`Located<String>` field selector. Both use the ordinary validation, free-name,
type-discovery, evaluation and copy APIs. For example:

```rust
use zirql::{Bindings, Budget, ConstructedValue as V, Located, ProgramLimits,
    ValueExpression as E, ValueExpressionKind as K, evaluate_value};
fn node(kind: K) -> E { Located::new(kind, 0..1) }
let expression = node(K::Windows {
    items: Box::new(node(K::Literal(V::Array(vec![
        V::Integer(1), V::Integer(2), V::Integer(3),
    ])))),
    size: Box::new(node(K::Literal(V::Integer(2)))),
});
let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 1_000, nesting: 16, stream_items: 100,
});
assert_eq!(evaluate_value(&expression, &Bindings::default(), &mut budget).unwrap(),
    V::Array(vec![V::Array(vec![V::Integer(1), V::Integer(2)]),
                  V::Array(vec![V::Integer(2), V::Integer(3)])]));
```

The [value guide](values.md#complete-windows-and-consecutive-runs) specifies
positive sizes, complete stride-one windows, structural run keys and missing
fields. The shared reader lowers calls and `windows(size)`/`runs_by("field")`
suffixes to these same nodes. `ValueSuffix::Windows` retains a boxed size
expression; `ValueSuffix::RunsBy` retains a located field label. Suffix lowering retains dictionary continuations after tally; preparation
checks their kinds. Dispose retained native trees with `dispose_native_value`.

`FunctionRequest` uses existing positional and record views plus list/record
constructors to retain native members and source coordinates. A field key can
remain native when the host supplies its existing equality operation. Opaque
native selections need an explicit positional projection first. Hosts still own
handle admission, identity and rendering. No new public host hook is required.
The [local examples](examples.md#compare-consecutive-runs-with-global-groups)
run today; [consumer adoption](host-integration.md#sequence-neighbourhood-adoption)
is a separate integration step. These are ordinary value operations and add no
symbolic CP support.

## Immutable record expressions

Build `ValueExpressionKind::With { record, fields }` with a boxed base expression
and ordered `(Located<String>, ValueExpression)` replacements. The labels are
static existing string fields. This callable example returns a new capacity:

```rust
use zirql::{Bindings, Budget, ConstructedValue as V, Located, ProgramLimits,
    ValueExpression as E, ValueExpressionKind as K, evaluate_value};
fn node(kind: K) -> E { Located::new(kind, 0..1) }
let expression = node(K::With {
    record: Box::new(node(K::Literal(V::Record(vec![
        ("limit".into(), V::Integer(8)),
        ("used".into(), V::Integer(6)),
    ])))),
    fields: vec![(Located::new("limit".into(), 0..1),
                  node(K::Literal(V::Integer(12))))],
});
let mut budget = Budget::for_program(ProgramLimits {
    work: 10_000, results: 1_000, nesting: 16, stream_items: 100,
});
assert_eq!(evaluate_value(&expression, &Bindings::default(), &mut budget).unwrap(),
    V::Record(vec![("limit".into(), V::Integer(12)),
                  ("used".into(), V::Integer(6))]));
```

The common reader lowers `record with {field: expression, ...}` to this node.
Validation, free-name discovery, type discovery and execution use the ordinary
value APIs. Preflight checks every replacement without installing field names
as locals. Runtime evaluates the base once and replacements in written order,
then rebuilds the base layout and admits the result against retained contracts.
The [value guide](values.md#reuse-a-record-with-changed-fields) describes refusals
and includes a runnable original/updated report.

`FunctionRequest` uses the existing constructed views or
`ProgramHost::capture_record` to select actual children.
`ProgramHost::construct_record` builds a native result when needed. Unchanged
children retain their owners and source coordinates through those boundaries;
no native handle is converted to text. `ProjectionHost::capture_record_kind`
and declaration kinds describe the result for later field use. No new public
host hook is required. Traversal, metadata, construction and copies share the
active budget; failure returns no partial record. Retained native expression
trees use the existing `dispose_native_value` path.

This form replaces existing fields only. General dictionaries, keyed arrays,
source edits and symbolic CP lowering require separate support. See
[consumer adoption](host-integration.md#immutable-record-adoption) for Zincite
and Zirium integration requirements.

## Record alignment expressions

Build `ValueExpressionKind::Align { left, right, field }` with two boxed
positional record expressions and a `Located<String>` field selector. The
selector range locates missing, absent and duplicate key errors. This callable
example compares two one-row tables:

```rust
use zirql::{Bindings, Budget, ConstructedValue as V, Located, ProgramLimits,
    ValueExpression as E, ValueExpressionKind as K, evaluate_value};
fn node(kind: K) -> E { Located::new(kind, 0..1) }
fn table(ms: i64) -> E {
    node(K::Literal(V::Array(vec![V::Record(vec![
        ("id".into(), V::String("parse".into())),
        ("ms".into(), V::Integer(ms)),
    ])])))
}
let expression = node(K::Align {
    left: Box::new(table(12)), right: Box::new(table(10)),
    field: Located::new("id".into(), 0..1),
});
let mut budget = Budget::for_program(ProgramLimits {
    work: 100_000, results: 10_000, nesting: 64, stream_items: 100,
});
let V::Array(rows) = evaluate_value(&expression, &Bindings::default(), &mut budget)
    .unwrap() else { panic!("aligned rows") };
assert_eq!(rows.len(), 1);
let V::Record(fields) = &rows[0] else { panic!("aligned record") };
assert_eq!(fields[0], ("key".into(), V::String("parse".into())));
```

Validation, free-name discovery, type discovery and execution use the ordinary
value APIs. The common reader lowers exactly `align(left, right, by="field")`
to this node. Both operands are prepared even for empty input, then evaluate
once in written order. [The value guide](values.md#align-two-record-sequences)
defines key identity, ordering, absence and refusals; the [CSV comparison](examples.md#compare-benchmark-tables)
shows guarded downstream field access. Dispose retained native expression trees
with `dispose_native_value`.

`FunctionRequest` uses existing positional and record views, structural
hashing/equality and list/record constructors. It preserves original records,
native owners and source coordinates. Result kinds retain optional side
contracts for downstream projections, including typed empty sequences. An
untyped empty source supplies no record schema. Hosts keep native identity,
admission and rendering policy. No new host hook is required. This eager value
operation has no pipeline suffix, selector callback or symbolic CP lowering.
See [consumer adoption](host-integration.md#record-alignment-adoption) before
advertising it through a Zincite or Zirium frontend.

## Render retained reports

Keep native results in the program's value type until the caller requests output.
Implement `RenderHost` to expose borrowed report sequences, records and captures,
and project native leaves with their owner checks. `to_report_json_value` handles
container traversal and the selected constructed JSON representation. Pass the
same request budget to that conversion and `render_json_value`; add document or
completeness fields between those calls. The [rendering guide](library-rendering.md#mixed-native-reports)
contains a complete mixed-report example and describes the allocation boundary.
Evaluation never needs a JSON conversion.

## Collection bounds in a value tree

Build `ValueExpressionKind::Head { collection, count }`, `Tail`, `Reverse` and
`Check { collection, operand, message }` directly. `count`, `operand` and `message`
are ordinary expressions; use `None` for an omitted check operand/message.
`Filter` keeps the original collection kind and tests dictionary values.
`ExpressionStep::{Head, Tail, Check}` and constant `Stage` operands use the same
admission and operations for native hosts. Existing `ProgramValue` and native
query leaves need no additional predicate payload API.

Run a constructed chain with the word adapter:

```sh
cargo run --locked --example words -- \
  '[3,1,3] | filter(_ > 1) | head(length([0,0])) | reverse | check(2, "two rows")'
```

It returns `[3, 3]`. The [collection reference](library-pipelines.md#filter)
defines empty behavior and the explicit keyed-array conversion boundary.

Native unique and tally use `evaluate_native_unique_tally` through the
`ProjectionHost::collection_unique_tally` delegation. Keep actual keys in the
host dictionary view and expose that storage through `RenderView::Dictionary`
when rendering. A source owner or diff side stays with each native key through
saved bindings, copies, lookup and dictionary continuations.
