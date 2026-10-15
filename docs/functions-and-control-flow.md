# Functions and control flow

A word analysis might need to handle empty input, distinguish a missing label
from a present one, or reuse a calculation for several groups. Conditions,
patterns and functions express those choices. Bindings are immutable, and
functions receive outer data through explicit arguments.

Conditions and value matches below run in the [word
example](getting-started.md). Function declarations need a host that registers
and prepares functions before evaluating a call. The common reader parses those
declarations, but the small word executable accepts one expression and does not
install functions.

## Conditions and local bindings

A conditional has a required final `else`. Each branch is a block with optional
local bindings and a final value:

```zirql
if length(words) == 0 {
  "no words"
} else {
  let first = words[1];
  "first: " ++ first
}
```

This returns `"first: ash"`. Only the selected block executes. Its names stay
inside that block. An initializer sees the outer scope and earlier bindings;
it cannot refer to its own new name. Duplicate names and shadowing fail.

Use `else if` for another condition. Every written branch receives scope, type
and purity checks, even when its condition is false. Runtime errors in an
unselected branch, such as a missing array position, remain unexecuted.

## Select a value with patterns

A value match evaluates its subject once, then tries arms in written order:

```zirql
match (length(words)) {
  0 => "empty",
  1 => "one word",
  n if n < 10 => "a few words",
  _ => "many words"
}
```

The result is `"a few words"`. A name pattern binds the subject for that arm's
guard and result. A false guard continues to the next arm. The final arm must
be an unguarded `_` wildcard.

A pattern mismatch tries the next arm. A guard error does not. Guards require
known Booleans; unknown native truth and non-Boolean values fail. All arms
receive preflight checks, and only the selected result executes.

## Destructure records and tuples

Patterns can select fields, rename them and ignore unused data:

```zirql
match ({"word": "apple", "count": 2}) {
  {word: spelling, count: n} if n > 1 => spelling,
  _ => "no repeated word"
}
```

This returns `"apple"`. `{word}` is shorthand for `{word: word}`. A final `..`
permits additional stored record fields without binding them. Without `..`,
the record's stored labels must match the pattern exactly.

Tuple patterns require a tuple of exactly the given length. `(name, _)` binds
the first value of a pair and ignores the second. It does not match an array.
A one-element tuple is written `(value,)`; `(value)` groups an expression.
Literal patterns require the same kind, so an integer pattern does not match a
float with the same numeric magnitude.

Local `let` bindings can use these patterns too:

```zirql
if true {
  let {word: spelling, ..} = {"word": "ash", "count": 1};
  spelling
} else {
  "unreachable"
}
```

The result is `"ash"`. A failed binding pattern is an error, and none of its
names is installed. All new names become available together after a successful
match.

## Match optional values

Use `absent` and `present(pattern)` to inspect an optional value:

```zirql
match (lookup({"word": "ash"}, "label")) {
  absent => "unlabelled",
  present(label) => label,
  _ => "unlabelled"
}
```

This returns `"unlabelled"`. `present` views an existing present value; it does
not introduce another wrapper. The final wildcard is required even when the
other arms cover the expected cases.

A missing record field and a stored absent field remain different shapes.
A record pattern requires its named fields to be stored. To treat a missing
field as optional data, use `lookup` before matching it.

## Declare a function

A function names its parameters and result type:

```zirql
fn selected_count(xs: sequence<string>, prefix: string) -> int {
  length([w for w in xs if starts_with(w, prefix)])
}
```

In a host that supports functions, `selected_count(words, "a")` returns `3` for
the word model. Arguments are positional and evaluated left-to-right. Every
actual argument must meet its complete declared type before parameter patterns
bind. The returned value must meet the result type.

| Type spelling | Contract |
| --- | --- |
| `int`, `float`, `bool`, `string` | The corresponding scalar kind |
| `opt T` | A present `T` or absence |
| `sequence<T>` | A positional sequence whose elements satisfy `T` |
| `tuple<T, U>` | Two fixed positions with their own types |
| `record {word: string, count: int}` | Declared record fields |
| `set<T>` | A supported scalar or nominal set |
| `keyed<T>` | Keyed cells satisfying `T` |
| `queue<T>` | A computational FIFO with elements satisfying `T` |
| `identity_set<T>` | A computational set of one native kind |
| A registered name | A native or nominal type resolved by the host |

Optional record fields may be omitted; extra labels fail the declared contract.
Integers do not implicitly satisfy a float contract. Native declarations retain
the host's ownership and identity checks.

A host using the pattern declaration reader can destructure a typed parameter:

```zirql
fn repeated({word: spelling, count: n}: record {word: string, count: int}) -> string {
  if n > 1 { spelling } else { "" }
}
```

Functions have no default arguments, closures, imports, overloads or function
values. Names register before bodies, allowing mutual recursion. Functions
cannot edit or emit, and domain operations called by a function must be pure.
External `param` inputs to a program are a separate mechanism with a smaller
set of types; see [program parameters](native-programs.md#required-caller-parameters).

## Recursion and limits

A function can call itself or another function in the same prepared request:

```zirql
fn count_down(n: int, total: int) -> int {
  if n <= 0 { total } else { count_down(n - 1, total + 1) }
}
```

`count_down(4, 0)` returns `4`. The recursive call is the selected final result,
so compatible result contracts allow it to reuse the current activation.
An expression such as `1 + count_down(n - 1, 0)` has a pending addition and
needs another active function call.

Tail-call reuse does not remove work limits. Each call spends work, and an
endless recursion eventually fails. Expression nesting and native callback
depth have their own limits. See [diagnostics and limits](diagnostics-and-limits.md).

## Other control-flow forms

The evaluator also supports folds, scans, state loops and worklist collections.
The common expression reader does not parse these operations. A host must
translate its own syntax into the Rust representation, or construct that
representation directly.

| Form | Behavior |
| --- | --- |
| Fold | Apply a named pure step to each item and return the final state; empty input returns the initial state |
| Scan | Retain each post-step state; exclude the initial state and return an empty sequence on empty input |
| State loop | Repeatedly compute a complete next state or break with a result |
| FIFO queue | Pop the front or append items while preserving existing aliases |
| Native identity set | Insert a native identity or test membership |

Fold and scan require a step with state and item parameters. The state contract
is fixed; every returned state is checked. A loop likewise computes all next
state values against the old state before replacing any of them. Failed checks
cannot leave partially updated state. Neither repetition nor an empty input
bypasses preflight or admission.

Queues and identity sets retain native ownership. They are computational
values, with no scalar serialization or identity-set iteration. The implementation
reference covers [folds and scans](embedding-programs.md#fold-and-scan-named-functions),
[state loops](embedding-programs.md#repeat-with-explicit-state) and
[native collections](value-runtime.md#native-computational-collections).
