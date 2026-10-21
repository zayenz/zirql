# Functions and control flow

The shared reader supports these forms; the host chooses which complete
programs, typed functions and output statements it installs. The word and CSV
examples use `ConstructedProgram` for named functions, typed parameters, ordered
lets and a final value expression. CSV supplies caller
parameters through repeated `--param name=value` arguments; the word executable
supplies no external parameter values. See [functions and control flow](functions-and-control-flow.md)
and [programs](native-programs.md).

## let

`let pattern = expression;`

Bind an immutable value for later expressions. `let n = 2; n + 1` returns `3`
in a frontend accepting ordered lets. Initializers see outer names and earlier
bindings; new pattern names become available together after a successful match.
Duplicate names, shadowing, forward/self references and strict pattern mismatches
fail. Local lets stay in their function/control block.
See [binding scope](native-programs.md#statements-and-scope).

## fn and calls

`fn name(pattern: T, ...) -> R { let ...; expression }`; `name(argument, ...)`

Declare and call a named pure function. `fn twice(n: int) -> int { n * 2 }`
followed by `twice(3)` returns `6` in a supporting host. Arguments run left to
right and satisfy complete declared types before patterns bind; results satisfy
the declared return type. Functions receive outer data through explicit arguments.
Names register before bodies, permitting mutual recursion; limits still bound
work and active calls. No defaults, closures, overloads or function values exist.
Builtin names are reserved; domain calls need host registration.
See [declarations and types](functions-and-control-flow.md#declare-a-function).

## if

`if condition { value } else if condition { value } else { value }`

`if condition then value elseif condition then value else value endif`

Conditions require known Booleans; the final `else` is required. Only the chosen
branch executes, but every branch receives scope, known-type and purity checks.
`if false { [][1] } else { 7 }` returns `7`. Blocks permit ordered local lets
and one final expression, whose names stay local. Branch result contracts still
apply. See [conditional blocks](functions-and-control-flow.md#conditions-and-local-bindings).

## Value match

`match (subject) { pattern if guard => result, ..., _ => fallback }`

Evaluate the subject once and select the first matching pattern with a true
guard. `match (2) { 0 => "none", _ => "some" }` returns `"some"`.
The last arm must be an unguarded wildcard. A pattern mismatch or false guard
tries the next arm; guard errors, unknown truth and non-Booleans fail.
Patterns include literals, names, `_`, tuples, positional lists, records,
`absent` and `present(pattern)`. List/record `..` permits an unbound remainder;
tuple arity stays exact. Arm names are local; every written arm receives
preflight checks. Native `match { ... }` instead enumerates captures.
See [pattern matching](functions-and-control-flow.md#select-a-value-with-patterns).

## fold

`fold(items, initial_state, step_name) -> final state`

Apply a named pure function with state and item parameters to an eager positional
sequence in order. With `fn add(s: int, x: int) -> int { s + x }`,
`fold([2, 3], 0, add)` returns `5`. Empty input returns the initial state.
The state contract stays fixed and each returned state is checked. The step
position accepts a name, not a closure. Preflight still checks empty input.
See [named reducers](functions-and-control-flow.md#other-control-flow-forms).

## scan

`scan(items, initial_state, step_name) -> positional array of states`

Use the same state/item step and checks as fold, retaining each post-step state
in traversal order. With the `add` function above, `scan([2, 3], 0, add)` returns
`[2, 5]`. Exclude the initial state; empty input gives `[]`.
See [named reducers](functions-and-control-flow.md#other-control-flow-forms).

## loop

`loop (state = initial, ...) { body } -> break result`

Introduce fresh immutable state names and repeat a body whose every path ends
with `break value` or `continue (next, ...)`.
`loop (n = 2) { if n == 0 { break n; } else { continue (n - 1); } }`
returns `0`. Initializers run in surrounding scope; fixed state contracts apply
to every transition. Repetition spends the request budget and does not return
a partial result on exhaustion. See [state loops](functions-and-control-flow.md#other-control-flow-forms).

## break

`break expression`

Finish the innermost loop in the current function and return the expression's
value. `loop () { break 7; }` returns `7`. A transfer can end with a semicolon
inside its block. Break outside an applicable loop fails admission; it does not
escape into a caller's loop. See [loop rules](embedding-programs.md#repeat-with-explicit-state).

## continue

`continue (next_state, ...)`

Compute all next values against the old loop state, then replace every state
slot together. The value count and fixed state contracts must match.
`loop (n = 1) { if n == 0 { break n; } else { continue (0); } }` returns `0`.
Failed checks leave no partial transition. Continue requires the current
function's innermost loop. See [loop rules](embedding-programs.md#repeat-with-explicit-state).

## param

`param name: int;`, `param name: string;`, `param name: bool;`

Require a typed external value in a host program. `param limit: int;` admits
the caller's integer without source-text substitution. Missing, extra, duplicate
or incompatible values fail before domain execution. Parameters have no defaults
and become visible at their written declaration. Hosts document their CLI/API
input handling; this is separate from function parameter types.
See [caller parameters](native-programs.md#required-caller-parameters).

## report

`report expression;`

The shared statement reader recognizes an output statement; the host decides
rendering, library output and terminal/pass-through policy.
`report {"count": length([1, 2])};` asks a supporting host to report count `2`.
Pure functions and bindings cannot emit or edit. CSV uses its final expression
as implicit JSON output and does not accept this statement.
See [output boundaries](native-programs.md#written-and-implicit-output).
