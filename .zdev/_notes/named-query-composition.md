# Reusable named native queries

Base-108, 2026-10-21. This design selects existing typed pure functions for
reusable native selections. It changes no runtime, reader, API or consumer code.

## Selected scope

The user instructed: “Follow your recommendations and set zdev goal again.”
That delegates following the recommendation to reuse existing functions. It
does not approve an unseen `query` declaration or pipeline-call syntax. Keep
explicit function calls and save complete programs through existing host APIs.

## Zincite: decision declarations

This current-language program takes syntax nodes and returns semantic declarations:

```zirql
fn decision_declarations(nodes: sequence<Node>) -> sequence<Declaration> {
  nodes | filter(kind("declaration")) | declarations | filter(instantiation("var"))
}
let selected = decision_declarations(items);
selected | names
```

`nodes` is an explicit ordered syntax-node sequence from the current input.
`kind("declaration")` retains declaration syntax; `declarations` changes the
native kind to semantic declarations; the final filter retains proven `var`
declarations. The declared result is a native declaration sequence, not names
or rendered text. The caller explicitly projects names after the call.
Zincite owns model loading, native identity and coordinates, model facts,
completeness and Unknown handling. An incomplete selection is not proof that
other declarations are parameters. Reuse does not weaken those host rules.

## Zirium: expensive users

This current-language program takes producers and a minimum cost and returns users:

```zirql
fn expensive_users(producers: sequence<Operation>, minimum: int) -> sequence<Operation> {
  [user for producer in producers for user in user_info(producer).targets
        if (cost_annotation(user) default 1) >= minimum]
}
let producers = [producer for producer in filter(op("graph.start"))];
expensive_users(producers, 3) | names
```

The explicit comprehension constructs the typed native operation sequence.
Zirium distinguishes ordinary operation streams from native `OperationSequence`
values at admission; do not replace it with `let producers = filter(...)` and
assume that stream satisfies `sequence<Operation>`. The result retains checked
operation handles until the caller projects names.
The function preserves producer order, each producer's user order and repeated
use sites. `user_info` orders targets by result index then SSA use-site order,
including successor arguments. It requires complete adjacency and reports a
located error when that is unavailable. Missing cost means 1; malformed,
non-integer or out-of-range cost remains an error. Zirium owns these facts.

## Calls and alternatives

`function(source, args)` supplies every input explicitly. Functions see their
parameters, earlier immutable locals, registered functions and pure operations;
they have no ambient pipeline input or captured top-level bindings/parameters.
Signatures register before bodies. Arguments evaluate once in written order,
then pass whole-type admission; results pass their declared contracts. Preflight
checks every written body and branch for scope, type and purity, even on empty
input. Functions cannot emit or edit. Calls, native operations, copies and
rendering share request budgets.

| Form | Consequence | Decision |
| --- | --- | --- |
| Existing `function(source, args)` | Explicit input, arity and native transitions use current rules. | Use it. |
| Hypothetical `query name(...) -> Type { body }` alias | Keeping explicit arguments and function rules adds no capability. An implicit input would need a new contract. | Defer it. |
| Hypothetical `source | function(args)` | First-argument sugar needs parser dispatch, current-input binding and arity rules; the source must evaluate once and retain whole admission. | Defer it. |

Neither alternative warrants another callable system. A pipeline's result kind
may differ from its input kind, as Zincite demonstrates; explicit signatures
already describe that transition. Repeated calls evaluate again. For reuse of
one result, write `let selected = function(source, args);` and use `selected`.

## Smallest saved workflow

Save one complete program with its functions, any required scalar `param`
declarations, ordered lets and final expression. Run it with
`zincite query --program-file decision-declarations.zq model.mzn` or
`zirium --program-file expensive-users.zirql graph.mlir`. Existing host input
options still apply. A caller parameter must be passed explicitly, for example
`expensive_users(producers, minimum)` after `param minimum: int;` at top level.
No named entry point is required. Library callers may retain the parsed host
`Program`, but each evaluation prepares against its current owners and facts.
Keep prepared function state within that evaluation. Defer imports, definition
libraries, package management, general closures and pipeline sugar.

## Documentation and follow-ups

There are zero runtime follow-ups. If useful, propose one Zirql documentation
slice for explicit native inputs and saved programs in the function, native
program and embedding guides. Propose Zincite and Zirium cookbook examples
separately under their host authority. Import no implementation tasks here.
CP remains an optional ordinary-value boundary; these native functions do not
become symbolic CP operations.

## Review evidence

The examples are source-reviewed, not executed during this design task. Checked
`src/functions.rs`, `src/program.rs`, `docs/functions-and-control-flow.md`,
`docs/native-programs.md` and `docs/embedding-programs.md`; Zincite's
`docs/reference.md` and `docs/query-cookbook.md`; and Zirium's
`docs/query-language.md` function, subtree-cost and worklist sections plus native
sequence admission in `crates/zirium/src/query/functions.rs`. The coordinator
runs `git diff --check` and `zdev check base --format json` after integration.
No Rust tests are needed for this design-only change.
