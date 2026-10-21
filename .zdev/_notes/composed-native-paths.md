# Composed native paths

## Decision and status

On 2026-10-21 the user accepted the recommendation with “Follow your
recommendations and set zdev goal again”: select composition and bounded
repetition first; defer named intermediate captures. This records base-105's
design decision. The syntax below is proposed, except where marked current.
It does not add runtime support or authorize consumer edits.

Prefer this short form:

```zirql
paths(defs / parent{1,2}, to=op("func.func"), max_depth=3)
```

`/` sequences primitive relations. Each term may have finite `{min,max}`
repetition, including a standalone `referenced_declarations{2,3}`. A bare term
inside a sequence means exactly one hop. A top-level bare primitive keeps its
current meaning: zero through `max_depth` hops. Thus `paths(defs, ...)` retains
reachability, while `paths(defs / parent, ...)` requires both edges in order.
Bounds are checked nonnegative integer literals with `min <= max`; malformed,
negative, overflowing or reversed bounds refuse with the term's query range.

The alternative `sequence(defs, repeat(parent, min=1, max=2))` states the same
relation but adds nesting and words to an ordinary command-line query. Defer it.
Also defer alternation, grouping, unbounded repetition, arbitrary query stages,
heterogeneous kind transitions and named captures. This is no full graph language.

## Two host journeys

In `@mix` in Zirium's current `examples/cli/function-report.mlir`, the return's
operand is defined by `arith.addi`, whose parent is `func.func`. With that return
selected, the proposed form above yields native steps
`[func.return, arith.addi, func.func]`. It uses one definition edge and one parent
edge; the optional second parent would reach the module and not match this target.
The current `defs | parent` pipeline finds the function but loses the journey.
A manual loop would retain a phase, current operation, depth and accumulated
steps while invoking the existing navigation stages. Composition supplies that
bounded traversal and witness without a query-specific worklist.

The useful intermediate capture is the defining `arith.addi`: a report may want
its name and location alongside its enclosing function. `steps` already retains
it as a native operation, so the first slice needs no capture syntax. A later
capture design must decide which repeated occurrence and segment boundary a
name denotes; no implicit “first” or “last” capture is selected here.

Zincite's current `docs/queries.md`, “Explain declaration connections”, uses
this model:

```minizinc
int: base_capacity = 10;
int: usable_capacity = base_capacity;
int: daily_limit = usable_capacity;
solve satisfy;
```

This existing query is runnable in Zincite:

```zirql
subtree | declarations | filter(name("daily_limit"))
| paths(referenced_declarations, to=name("base_capacity"), max_depth=2)
| json
```

The proposed relation `referenced_declarations{2,3}` selects two or three
reference edges, returning `[daily_limit, usable_capacity, base_capacity]` here.
The existing pipeline `referenced_declarations | referenced_declarations` finds
the endpoint but drops `usable_capacity` as an explanatory step. Zincite's
published `reference_info` worklist supports explicit queue/identity-set logic;
it would need predecessor and depth state to return this bounded witness.
`usable_capacity` is the useful intermediate declaration and remains in `steps`.

## Relation and witness contract

Each primitive preserves one native kind and owner. Future Zirium admission is
unindexed `defs`, `users`, `parent` and `children`, all Operation → Operation;
Zincite remains Declaration → Declaration through `referenced_declarations`.
Preflight validates every term, repetition bound and target branch before any
scan, including empty input and `{0,0}`. Reject indexed arguments, projections,
effects and unsupported primitives. No term changes document or diff side.

Accept a target only after the whole relation accepts. `max_depth` bounds total
native edges across all terms, not each term separately. A relation whose
minimum length exceeds that bound returns no witness. A zero-edge witness
exists only when every required term permits zero and the source matches the
target. Its steps are `[source]`; segment changes add no artificial duplicates.
Rows retain the existing native `source`, `sink` and ordered `steps` schema.

Choose one shortest accepted witness per source/sink pair by total edge count.
Distinct sources retain incoming identity order; sinks use breadth-first
discovery order, with host neighbor order resolving equal-length witnesses.
For an ambiguous segment decomposition, advance to the next segment once the
minimum is met before taking another optional repetition. That tie rule is
observable if a future feature exposes segment boundaries.

Traversal state includes native identity plus segment and repetition progress.
A node-only visited set would wrongly discard a return to the starting identity
through `defs / children`; arriving at a different relation position can matter.
Suppress redundant traversal states and deduplicate final source/sink pairs;
steps may contain actual cycle revisits. Finite term bounds, total `max_depth`
and the continuing request budget bound the search. Do not enumerate all paths.
Charge parsing, validation, scans, comparisons, state storage, native copies,
witness construction and rendering to the existing budget; exhaustion is an error.

Keep Zincite's path policy: proven adjacency can yield witnesses while unresolved
edges retain located limitations, `complete:false` and exit status 1. This differs
from `reference_info`, which requires complete adjacency or errors. A final
Unknown target still errors. A missing witness within the bound establishes no
global disconnection; unresolved routes may remain. Zirium paths stay read-only
and unavailable on diff queries. Both hosts retain owners, handles, source
coordinates and stale-handle checks until explicit rendering.

## Small implementation boundary

`src/reader.rs` would read a flat relation and bounds; `src/paths.rs` would hold
its typed terms/progress and extend the shared evaluator; `src/lib.rs` would
export the callable representation. The existing `PathHost` primitive hooks
suffice. Implementation must explicitly settle migration of the public
`NativePaths.edge` field and `read_paths` API; this note promises no silent
source compatibility. Zirium's `capture.rs` currently admits only `Users` and
ignores the edge in neighbor lookup: it needs real dispatch, plus parser and
builder admission, before any proposed Zirium example runs. Zincite needs reader
and callable adoption while preserving its partial-adjacency policy.

Propose three bounded follow-ups, without importing them in this design task:

1. Core reader/callable relation and evaluator, a runnable local example, and
   `docs/native-programs.md` plus `docs/host-integration.md`.
2. Separately authorized Zirium adapter/parser/builders and its
   `docs/query-language.md` plus `docs/query-dsl.md`.
3. Separately authorized Zincite adoption and its `docs/queries.md` plus
   `docs/reference.md`.

Future focused controls cover the composed witness, revisiting an identity at a
new segment, minimum/zero behavior, and invalid bounds or refused edges, such as
`defs(0) / parent`. Reuse existing owner, Unknown, incomplete-edge and budget
coverage. This design adds no product tests or public supported-syntax entries.
On `feature/gecode-cp`, `docs/constraint-programming.md` and `src/cp.rs` keep
ordinary preparation and concrete `yield` separate from symbolic expressions.
Paths may prepare ordinary values before `solve` or run in ordinary `yield`;
this proposal adds no symbolic traversal or dependency on that branch.
