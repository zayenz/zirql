# Shared query language foundations

## Objective

Build Zirql as a Rust framework for purpose-built query languages in Zirium
and Zincite. Provide a substantial shared base language, including query
execution and bounded comprehension evaluation. Consumers primarily adapt that
language to their domains through native types, operations and surface syntax.

Both tools use shared programs, bindings and comprehensions. Zirql owns their
common language execution; the adapters retain domain operations, native values
and surface forms that fit their users.

Observable success:

- Both tools use Zirql through callable library APIs and their existing CLIs.
- Existing queries, typed builders, native result types and documented host
  behavior remain compatible unless a separately approved change says otherwise.
- Both tools construct reports with comprehensions evaluated by the same core,
  using distinct surface forms where those fit better.
- Zincite uses that evaluator for constructed `.dzn` values and atomic edit
  candidates, subject to its existing target and write-safety requirements.
- Located diagnostics and finite work/result/nesting limits survive every host
  operation, nested expression and rendering boundary.

## User decisions

Confirmed on 2026-10-12:

- Name the project and framework `Zirql`, with `zirql` as the intended crate name.
- Include Zincite's planned comprehensions in the shared framework.
- Share evaluation rules; allow host-specific syntax where useful.
- Create a separate repository at `~/projects/zirql` and start GitHub private.
  Public release is a later decision, not part of this work.
- Track zdev project records; the `base` area follows `main`.
- Use `git-make-initial-commit` for the empty initial commit and `init` tag.

The user approved the exact ten-task bundle and authorized an implementation
goal on 2026-10-12. The outstanding Zincite data-edit contract choices retain
their attended gate.

The user subsequently authorized making the GitHub repository public on
2026-10-12, and that visibility change is complete. The user also authorized a
name-reservation prerelease followed by yanking that version. The user then
approved usable prereleases after independent verification when consumer
packaging requires a registry version. Preserve the yanked reservation.

On 2026-10-13, the user clarified that development should use pinned Git
dependencies. Reserve further publication for an explicitly chosen release
milestone; intermediate package validation does not trigger a prerelease.
Development integration uses workspace, library and CLI checks. Consumer
distribution packaging also needs sibling dependency version metadata and
registry readiness, which belong to release work. Keep existing usable
prereleases available.

On 2026-10-13, the user clarified that the base should contain more reusable
language behavior. Consumers should primarily supply domain adaptations, rather
than develop their own language extensions. Keep the implementation small by
sharing language machinery, not by pushing that machinery into adapters.
The user authorized retaining extension research and developing three example
programs for discussion. The specific extensions, syntax and task split remain
proposals; this exploration does not authorize their implementation.

The user subsequently endorsed the examples and requested a zdev task split
covering the full research scope, including grouping, keyed entry/alignment
helpers and explanatory paths. Short command-line programs are required. A
small query must not require a file, named entry point, type declaration or
parameter preamble when literal arguments suffice. Inline and file programs
use the same semantics, native results, diagnostics and limits.

The `zirql` name is reserved on crates.io by `0.0.0-prerelease`, published from
the verified core at `ab4aba4ade3100c3e3ebae23a98c6f56b72c6426` and yanked on
2026-10-12. This reservation does not supply a resolvable registry dependency.

The usable `0.1.0-alpha.1` prerelease was independently verified and published
from `7ee35cad2f8d92cbccf29977ff4496987cf7e19d` on 2026-10-12. Its registry
checksum matches the verified package; the name reservation remains yanked.

The usable `0.1.0-alpha.2` prerelease was independently verified and published
from `5dde88958b8e0744ca0e0b9814c19e88b614dedd` on 2026-10-12. The registry
archive matches the clean verified package byte for byte, and exact-version
resolution exposes the native program API. The reservation remains yanked and
`0.1.0-alpha.1` remains available.

The usable `0.1.0-alpha.3` prerelease was independently verified and published
from `4e4baf38e7c8566781f069dccea110383d6c0d24` on 2026-10-12. It preserves flat
left-associated native set sequencing without increasing grouped or recursive
nesting limits. Its registry archive matches the clean verified package byte
for byte, and exact-version resolution compiles. Earlier usable prereleases
remain available; the reservation remains yanked.

The usable `0.1.0-alpha.4` prerelease was independently verified and published
from `be8bd8b07d5e7e4fb148f8f2e032ffe0f5e226b9` on 2026-10-13. It supplies
budgeted scalar expressions and positional comprehensions through callable
validation, free-name, type-discovery, evaluation and copy APIs. Its registry
archive matches the clean verified package byte for byte, and an exact-version
registry consumer executes scalar/comprehension values and all five functions.
Earlier usable prereleases remain available; the reservation remains yanked.

The usable `0.1.0-alpha.5` prerelease was independently verified and published
from `bc7e8abdf928a27a4a56a049b17522eeefd680d4` on 2026-10-13. It adds nominal
members, typed sparse keyed arrays and keyed comprehensions to the five
budgeted value APIs, preserving positional values and host-owned shape policy.
The registry archive matches the clean reviewed package byte for byte, and an
exact-version registry consumer executes nominal/keyed values and all five
functions. Earlier usable prereleases remain available; the reservation remains
yanked.

On 2026-10-13, the user approved Zincite base-127's exact bounded data-edit
contract and chose `emit;` as its terminator, with `emit_document;` retained as
a compatibility alias. The approved source/let/create/replace grammar, closed
operator/helper subset, read-only model ownership, key/empty-axis rules,
comment/directive policy and explicit dependent-data mapping are recorded in
Zincite's `docs/query-comprehension-contract.md`. Zincite implemented that
contract in `4b4246d543b612a4b94264f96411a78b2dc16f74` and the terminator change
in `ae1ccdd9958ad633f20f6baf5d9665ce20101ef8`; its base-127 is complete.
Base-008 verifies those actual library and CLI outcomes against the current
shared dependency. Reuse the existing implementation; new semantic choices
outside that exact contract remain attended.

## Architectural direction

Start with one reusable Rust library crate. A framework executable, parser
generator, dynamic plugin loader, query optimizer and persistent cache have no
current consumer requirement. Use the consumers' MIT OR Apache-2.0 license
convention. Follow the development and release policy above. A stable 1.0 API
remains outside this increment.

Use [BAML](https://boundaryml.com/) as an additional design reference for readable
language syntax, explicit types and native host integration. Draw on its ideas
where they simplify Zirql's lowering, diagnostics or typed builder APIs; each
choice still needs a concrete Zirium or Zincite use.

### Base language and domain adaptations

Zirql owns reusable language constructs and their validation, scope, evaluation,
ordering, cardinality, diagnostics and resource accounting. New per-item record
projection, reducers, typed parameters, grouping, requirements and capture
composition, if selected, belong in the base. A concrete consumer use can
justify a shared construct before both consumers need it; duplicate host
implementations are not a prerequisite. Prefer direct shared implementation
over an extensibility system built solely to keep the core small.

Hosts supply domain types, primitive operations and predicates, native identity,
source coordinates, rendering and effect policy. For example, Zirql would own
capture bindings and match enumeration; Zirium would supply operand lookup and
SSA-value equality. Domain-specific spellings may lower to the same constructs.
The common frontend and typed builders should make base features reusable
without each consumer implementing their language rules again.

Read [the extension research](background/language-extensions.md) when comparing
new base capabilities or their ownership. Read [the three example
programs](background/language-design-examples.md) when discussing projection,
value aggregation, parameter syntax, requirements or native structural
captures. The examples expose unresolved choices and are not executable
contracts. The existing comprehension contract and approved task boundaries
continue to govern implementation until a design change is selected.

### Implemented extension bundle

The task bundle selects the examples' explicit `project(subject, {fields})`,
required `param name: type` declarations with `int`, `string` or `bool` types,
eager array reducers and conditional `require`. Initially `require` accepts a
literal string message; it preflights all operands and evaluates its value only
after a true condition. It includes shared `match` clauses (`from`, `bind`,
`where`) returning native capture records. Named result schemas, parameter
defaults, arbitrary functions and lazy generator reduction remain deferred.
The user approved this exact bundle on 2026-10-13. Tasks base-018 through
base-025 record the implemented scope. These forms are available through the
pinned development Git dependency; the published alpha.5 predates them.

Use Zirium's existing query argument for inline programs. Both Zincite query
commands accept `--program TEXT`, mutually exclusive with `--program-file` and
the legacy positional query. Both hosts accept repeated `--param name=value`
for declared inputs: strings are passed verbatim, integers are checked decimal
`i64`, and Booleans are exactly `true` or `false`. Duplicate, missing, extra or
incompatible parameters fail before domain execution. Library callers supply
typed values. Files remain useful for longer programs; single expressions and
pipelines keep their existing host emission conventions.

The shared forms include `group_by(rows, "field")` with first-seen
groups and typed keys; `entries(a)` for keyed cells; `zip_exact(a, b)` for
keyed arrays with identical ordered axes and present keys, including identical
sparse coverage, producing two-element tuple cells; and one deterministic
shortest path per reachable source/sink pair within an explicit edge-depth
bound. Hosts supply path edges and native identity. Their uncertainty and diff
side rules survive shared execution. Path queries are read-only and do not
enumerate all possible paths.

Path steps include both endpoints. Zirium's `users` edge means one operation
to one direct SSA user operation. Zincite's `referenced_declarations` edge
means one declaration to a proven target of a reference written anywhere in
its owning region, following `transitive_references`' existing region and
reference-order rules. Each such declaration-to-declaration connection counts
as one edge; reference occurrences are not additional steps. Repeated targets
are considered in first-reference order. Unknown target predicates cause a
located refusal, and unresolved edges retain the host's incompleteness.

For the MLIR fixture in the examples, this command has one witness
with steps `[arith.muli, arith.addi, func.return]` in `mix`:

```sh
zirium 'filter(op("arith.muli")) | paths(users, to=op("func.return"), max_depth=2) | json' input.mlir
```

For a model containing `int: c = 1; int: b = c; int: a = b; solve satisfy;`,
this command has one native declaration witness `[a, b, c]`. At depth
one it has no witness for `c`:

```sh
zincite-query --program 'subtree | declarations | filter(name("a")) | paths(referenced_declarations, to=name("c"), max_depth=2) | json' input.mzn
```

These extension tasks supplement base-007 through base-010; they do not broaden
those tasks' completion conditions. Only keyed helpers require base-007's
consumer integration. Preserve the existing attended gate for data edits.

### Syntax and executable representation

A surface frontend lowers syntax to a shared, located program representation.
That representation covers ordered statements, immutable bindings, pipelines,
nested expressions, predicates, common collection stages and value expressions.
Host stages and predicates retain typed host-specific payloads and argument
locations. The core does not recognize MiniZinc node kinds or MLIR operation
names.

Provide a reusable reader for common pipeline/program syntax and explicit host
extension points for operations and predicate arguments. Hosts may lower their
existing parser output directly when that avoids a second parser. Zincite's
MiniZinc comprehension frontend uses its existing CST. Both frontend paths
execute the same normalized comprehension representation.

Do not reduce extensibility to stage-name aliases: the two consumers need
different literal forms, comprehension notation, initial selections and
emission rules. Give those named owners rather than building a collection of
unrelated Boolean configuration flags. Freeze concrete Rust signatures only
after the first two native integrations demonstrate their ownership needs.

The controlling path is conceptually:

```text
host source -> host/common parser -> located program
            -> validation against supported host operations and value types
            -> shared execution with host context and one budget
            -> native result / host emission requests
            -> host rendering or validated edit candidate
```

Parsing and validation are read-only. Unsupported syntax, unavailable bindings,
invalid stage transitions and incompatible value operations receive located
errors before their effects can run. Runtime failures retain their expression
or host-source location. Parsing can also produce an executable expression
without any CLI or renderer; typed builders use that same path without text.

### Execution and native values

The shared evaluator owns pipeline sequencing, binding lookup and scope, nested
input, common collection transformations and comprehension expansion. Generic
per-item projection and record construction belong to this shared execution;
hosts own the domain projections used inside them, navigation, predicate leaves,
domain comparison where needed, source rendering and effectful operations.
The host API receives the active budget for scans, traversal, copying and
nested evaluation; opening a nested query does not reset the allowance.

Keep native selections and values native. A Zincite node can borrow retained
source; a Zirium operation can remain a document-scoped handle. No JSON
serialization or common document graph is required to pass through the core.
Handle lifetime, side identity in diff queries and stale-handle checks stay
with the host. A constructed value has an expression location, not an invented
input-source range.

Shared scalar/collection concepts include Boolean, signed integer, finite float,
string, null for hosts that support it, positional sequence, tuple, record,
string-keyed map, typed set and explicitly keyed array. Introduce each variant
with its concrete report or data consumer. Keep native host items distinct from
scalars and nominal enum members; conversion requires an explicit projection.

The core supplies checked scalar operations and generator mechanics. Hosts
declare a bounded supported subset and map their spellings to those operations.
Trusted, explicitly registered host operations may project data or perform a
declared effect; user-authored callbacks and arbitrary model execution are not
part of the language.

### Compatibility and host policy

| Concern | Shared owner | Host obligation |
| --- | --- | --- |
| Streams | Ordered collection machinery and stable `unique` | Identity, supported kinds and existing set ordering |
| Predicates | Short-circuit `not`/`and`/`or`, including Unknown | Predicate leaves and reporting of limitations |
| Statements | Ordered execution, immutable bindings and nested scope | Supported statements, initial input and reserved names |
| Emission | Explicit emission events and result sequencing | Zincite terminal emitters; Zirium pass-through emitters and implicit final output |
| Limits | One active counter through shared execution | Existing defaults, current per-stream limits, explicit new aggregate limits and host scan admission |
| Effects | Nested execution context and declared effect boundary | Zirium live edits; Zincite candidate edits, complete validation and atomic replacement |
| Structured output | Native constructed values | Zirium JSON/report conventions; Zincite tagged values, axes and completeness |

An unknown predicate never becomes true through negation. Existing Boolean-only
hosts return known truth values; the framework does not fabricate Unknown for
ordinary errors. Hosts keep incompleteness as distinct information, while type,
resource and runtime errors fail the request.

Preserve different current limits: Zirium uses per-stream item ceilings, while
new comprehension requests need aggregate expansion and copy accounting.
Expose those named bounds explicitly; retaining a legacy ceiling is not
permission to reset nested work counters. The host controls defaults. State
budget units and charge before allocation where an expansion size is knowable.

Pure expressions and bindings cannot edit or emit. A host effect follows its
existing transaction semantics. CLI requests buffer output until successful
completion; library callers retain their explicit emission/buffering contract.
This does not promise rollback for Zirium library edits after a later failure.

### Shared comprehensions

Read [the comprehension design](background/comprehensions.md) for examples,
generator rules, numeric behavior and the split between generic keyed
collections and MiniZinc target validation. MiniZinc surface spelling is
required for Zincite. Zirium extends its JSON-shaped
values with `for` and optional `if` clauses. Both lower to the same evaluator.

Zincite's existing comprehension contract is the compatibility baseline for its
subset. Its string-set exclusion and model-backed edit restrictions are host
rules; other hosts may support a value kind without inheriting MiniZinc
serialization restrictions. Host helpers such as enum-member resolution supply
native identities; they do not become built-in MiniZinc services in Zirql.

## Documentation site

On 2026-10-13, the user requested a Zirql documentation site similar to Zincite's,
at `https://zayenz.se/zirql/`.

Build up reader documentation alongside the language. Cover getting started,
shared language concepts and examples, Rust host integration, native results,
diagnostics and resource limits. Link to Rust API documentation and the consumer
guides for domain-specific syntax. Describe implemented behavior as available;
keep proposed language extensions clearly identified as proposals.

Use Zincite's repository-local Astro/Starlight site as the starting approach,
with authoritative Markdown guides in `docs/` and a separate build in `site/`.
Adapt its visual conventions to Zirql. Keep site tooling independent of Cargo.
When planning the site, inspect `../zincite/site/README.md`, `astro.config.mjs`
and `scripts/prepare-content.mjs`. When planning publication, inspect
`../zincite/site/PUBLISHING.md` and the personal site's current release rules;
Zincite currently imports its static build into the personal site's deployment.

Deliver reader guides, a locally reviewable site, then publication at the chosen
path. Validate the site build and manually check navigation, search, links,
assets, canonical URLs and mobile layout under `/zirql/`. Publication also
requires checks of the live HTTPS site and existing personal-site routes.
Use existing checks; documentation needs no new product tests. Reader guides
and the local site are complete in base-017. The user subsequently authorized
publication on 2026-10-13, and the site is published at `https://zayenz.se/zirql/`.
The [publication follow-up](tasks/017-build-zirql-reader-documentation-and-a-local-documentation-s.md#publication-follow-up)
records the imported source and website revisions.

The homepage, worked examples and design sketches use a small word model.
A separate semantic-model integration guide explains Zincite and Zirium's native
items, domain operations, identity, syntax and rendering. The docs header uses
light and dark SVG variants of the selected logo; the personal-site project card
uses the dark SVG.

The published copy describes its imported source revision. Later language or
documentation changes require a reviewed build and an explicit personal-site
import; a Zirql source push does not update the deployed site.

## Integration order

1. Demonstrate a small native pipeline through the core with two embedded host
   examples: borrowed source selections and document-owned operation handles.
2. Migrate Zincite's read-only pipelines, then Zirium's ordinary read-only text
   queries and typed builders. Retain host-specific navigation and rendering.
3. Share program sequencing, bindings, nested queries and structured results in
   both consumers, preserving Zirium's established syntax and adding Zincite's
   read-only program support.
4. Deliver positional comprehension reports through Zincite's CST frontend,
   then Zirium's JSON-shaped frontend, with the same scalar/generator evaluator.
5. Add enum-aware and multidimensional keyed collections for Zincite reports;
   make shape restrictions an explicit host validation step.
6. Use constructed values for Zincite data edits after its attended contract
   decision, and complete Zirium advanced diff/edit integrations separately.
7. Finish adoption with bounded compatibility and reuse measurements, usage
   documentation and removal of the shared implementations made redundant.

Each task delivers a narrow executable path. Core-example tasks establish an
API without claiming that a consumer has migrated. Consumer tasks verify actual
library and CLI paths; adapters never become dependencies of the core crate.

## Repository coordination

Zirql's base queue owns framework work and explicitly named integration slices.
Integration tasks may change Zirium or Zincite only when authorized for that
task. Read each repository's guidance and establish its Git baseline first.
Keep consumer commits separately reviewable and record their identifiers in
the task result; do not treat changes in one repo as a commit in another.

The user authorized coordination with the ongoing chat `zincite base zdev
implementation` (`01a0eebe-3bd0-78a0-b3a9-b87b8e4d02f7`). Check its current
progress and coordinate before overlapping source, planning-record or semantic
API changes. Begin with independent Zirql work; agree consumer ownership before
integration and avoid a second programs/comprehension implementation.

Zincite owns its semantic facts and source edits. Consume its public APIs and
preserve unrelated work. Its base-126 (read-only programs), base-127 (data
construction) and base-128 (measurement) have accepted consumer results. Reuse
that evidence with its recorded revisions and validate changed integration
paths; do not implement those outcomes twice. Do not mark a consumer task done merely because Zirql has an equivalent
core feature. Zdev task dependencies remain within an area; consumer readiness
is a precondition checked against the live repository, not a fabricated
cross-repository `blocked_by` entry.

## Testing

Use focused coverage of public behavior. Documentation and metadata need no
new product tests. The first core slice needs one compact control per host
ownership pattern. Later slices add only controls justified by their behavior:
binding scope, unknown negation, ordering, later-error output buffering,
cross-frontend comprehension equivalence, expansion limits, keyed-array shape
and atomic data replacement.

Reuse the host query suites. Preserve intentional fixture bytes and line
endings. Compare native results and observable outputs, not internal evaluator
call counts. Temporary instrumentation can establish parse/fact reuse without
permanent implementation-count assertions. No corpus campaign, solver run,
large generated test matrix or benchmark service is required.

For the language extensions, reuse the three example programs as compact public
behavior controls. Add focused checks for parameter/type refusals, projection
correlation and cardinality, eager reduction and lazy requirement failures,
native capture identity/scope, grouping order, key-alignment refusal and bounded
path witnesses. Each relevant integration demonstrates inline/file/library
agreement and existing later-error buffering. Reuse these controls across
surfaces; do not multiply them into a syntax matrix or add timing assertions.

## Validation

For record changes: `git diff --check` and `zdev check base --format json`.
Once Rust exists:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run each modified consumer's required checks. Use MiniZinc instance-only
acceptance for constructed public data; it supplements byte-preservation and
atomic-write checks and needs no solver. Measure only named successful cases;
report limit rejection separately from completed throughput.

## Reading sources

The planning inspection used Zincite at `ae1781fe932fbda306d1e7c077938db0a1a694bd`
with uncommitted semantic/query changes present, and Zirium at
`00f5cc6c1001cb60ac6dadb980d309e57449a702`. Inspect live sources before migration;
these observations are not a promise that a later checkout has the same APIs.

- Zincite: `~/projects/zincite/crates/zincite-query/src/{language,lib,semantic,structured}.rs`,
  `docs/reference.md`, `docs/query-comprehension-contract.md`, query library/CLI
  tests and `.zdev/base/brief.md`. Read the contract for expressions and data.
- Zirium: `~/projects/zirium/crates/zirium/src/query.rs`,
  `src/query/{lexer,parser,model,builder,diff,literal,render}.rs`,
  `docs/query-language.md`, `docs/query-dsl.md`, and query/diff tests. Read the
  builder and diff contracts before migrating their native results.
- [MiniZinc 2.10.1 specification](https://docs.minizinc.dev/en/2.10.1/spec.html)
  defines Zincite surface precedence, generators and keyed-array notation.

## Outstanding decisions

Concrete Rust signatures are chosen against the first two native controls within
these boundaries. The bounded Zincite data-edit contract's attended gate is
resolved as recorded under User decisions; broader data semantics need a new
decision. Base-008 records actual integration acceptance.
The repository is public and the crate name is reserved. Before another
publication or consumer distribution, follow the development and release
policy in User decisions.

The approved extension bundle records the selected concrete language choices.
Result schemas and broader function syntax remain future
design work. Shared base ownership and short command-line usage are settled.


## Next language design: types, absence and functions

On 2026-10-14, the user endorsed further language design covering the three
review suggestions below, host-defined domain types, optional values with a
MiniZinc-style default operator, and user-defined recursive functions. This
section records the design for the requested next task bundle. Approval of that
bundle does not extend the completion conditions of the current adoption work.

### Confirmed direction

- Keep checking mostly at runtime for now. Hosts describe their domain types
  and operation contracts; function parameters and results have explicit types.
  Retain existing preflight validation without requiring a new whole-program
  static type checker or local type-inference system.
- Support named, explicitly typed, pure query functions with bounded recursion,
  including calls to read-only domain operations. Function execution shares the
  request's work, result and nesting limits. Exhaustion remains a located error.
- Include presence checks and absence-specific defaults for sparse keys and
  optional fields. Defaults must not swallow arithmetic or budget failures.
- Include stable sorting and min/max for constructed reports. The user selected
  optional extrema results: absence for empty input and the first input item
  when keys tie. Callers can supply an absence-specific `default`.
- Include small shared string operations: contains, starts_with and ends_with
  for projected names.

### Proposed contract for discussion

Use existing scalar and collection concepts, structural report records and
nominal host types. For example, Zirium can describe operations and SSA values;
Zincite can describe nodes, declarations and semantic type handles. Hosts retain
native representations, identity and lifetimes. A query type describing a
MiniZinc declaration is distinct from the model-language type of that
declaration. Start with existing host kind and operation hooks; avoid building
a parallel registry unless the implementation demonstrates a concrete need.

Runtime contracts should check supplied arguments, returned values and executed
operations against declared types. Keep unknown validation information
(`ValueType::Unestablished`) distinct from a runtime absent value. Known host
uncertainty, an empty collection, missing data and execution failure remain
different outcomes. Existing heterogeneous constructed values need an explicit
compatibility decision before narrowing their accepted types.

Represent nullability as an optional type, conceptually `opt T`. The proposed
`x default y` evaluates `x` once and evaluates `y` only when `x` is absent;
a present false, zero, empty string or empty collection remains present.
Both selected values must satisfy the declared result contract. Provide an
explicit optional lookup/presence operation so existing strict field and key
access continues to report mistakes. Invalid key types and unknown declared
field names must not silently become absence. An omitted sparse cell and a
present cell containing absence must retain distinguishable membership.

MiniZinc's [option types](https://docs.minizinc.org/en/2.10.1/optiontypes.html)
provide the `opt`, presence and `default` reference. Its additional arithmetic
and comparison rules for absence are a separate choice. The recommendation is
to require explicit presence handling before ordinary arithmetic or ordering.
Surface spelling of absence may differ between hosts under the shared rules.

Functions should initially use explicit parameters and lexical scope. Native
arguments and results should remain native. Pure functions cannot emit or edit;
read-only host failures retain their meaning. Recursion must stop at an admitted
limit without overflowing the Rust stack, including on error and cleanup paths.
No termination proof is required. First-class functions, closures, polymorphic
inference and module loading are outside this initial proposal.

### Remaining choices and sequence

The task-bundle choices below make the remaining surface and behavior decisions
reviewable together. Extrema's optional empty result and first-in-input tie
selection and mandatory tail recursion are user decisions. Other choices below
are conservative defaults proposed for this bundle, not implemented behavior.

Refine host/runtime type contracts and optional values together, then typed
functions. Sorting/extrema and string helpers remain independently useful work;
their inclusion does not depend on finishing recursive functions. Keep short
inline queries free of mandatory declarations. Before task creation, show one
concrete function example per consumer and focused acceptance cases for absence,
wrong runtime types, recursive limits, stable ties and string operations.


### Execution model and efficiency proposal

On 2026-10-14, the user requested reasonable computational efficiency and a
well-considered execution model alongside the language additions. The following
is the proposed implementation direction, subject to the language choices above
and focused measurements. It does not authorize a runtime rewrite.

The current implementation supplies concrete starting points: `value.rs`
preflights expressions before running them, `projection.rs` invokes that path
again for each projected field and subject, local value reads scan names and
copy constructed values, and `Bindings::with_local` copies the outer name map
while sharing its native values. Function calls must not multiply these costs.
These are source observations, not measured performance regressions.

#### Prepare once, execute with runtime contracts

Use the conceptual path:

```text
host syntax / typed builder
    -> located shared representation
    -> prepared program: resolved names, calls and host operations
    -> interpreter: local slots, continuations and function call frames
    -> native / constructed result
    -> host emission
```

Preparation resolves local names to slots and function names to stable entries,
checks arity, lexical scope and pure/effect boundaries, and retains existing
preflight guarantees. Register declared signatures before preparing bodies so
self-recursion does not require recursively analysing a function's body at each
call. The selected task proposal includes mutually recursive named functions.
This preparation
is name resolution and structural checking, not a new static type-inference
requirement. Never validate a recursive call by expanding its body.

Retain source ranges on prepared operations and calls. Execute each ordinary
argument once in written order. Check argument and result contracts at runtime;
check data-dependent access, shape and host results when encountered. Avoid
re-walking a function body or rebuilding unchanged scope/type metadata for
every call or report row. Immutable values already checked against a contract
can retain that fact within the request; externally supplied or newly built
values still need admission. A type tag alone cannot establish every element's
type in an arbitrary host-supplied collection.

Start with a compact resolved representation executed directly. Node indices
and flat storage are candidates where they simplify execution and destruction;
there is no requirement for a general bytecode instruction set, optimizer or
JIT. Keep public parsing/building entry points usable through this path and
avoid maintaining two independent language evaluators.

Prepare once per submitted program and reuse function bodies throughout that
execution. Library reuse across requests is desirable only for preparation
independent of document values and host state. Initially bind prepared domain
payloads to the context that produced them; never reuse captured handles,
dynamic enum identities or inferred shape facts against a different document.
Preparation and fresh parameter admission remain bounded even if later runs
reuse a prepared representation.

#### Calls, recursion and native operations

Use explicit interpreter call/continuation frames for user functions, storing
the return position, local slots and query call location. Language recursion
must not grow the Rust call stack once per function invocation. Frames share
one request budget and release local temporaries on return or failure.

The user requires tail recursion in the first function release. A tail call
reuses its caller's frame and does not increase active function depth; every
call still consumes work. Non-tail calls consume active depth, and expression
nesting and host reentry retain their separate guards. This is an explicit new
function contract; existing expression limits and defaults remain unchanged.
Tail position includes the final expression and selected conditional branches,
requirement values and default fallbacks when no return conversion/check needs
a retained continuation. Guarantee frame reuse for self recursion and mutual
tail calls with the same declared result contract. Evaluate and check all new
arguments in written order before replacing locals. Do not retain a growing
chain of old frames, values, deferred return checks or diagnostic entries.
Validate and dispose prepared bodies and runtime values without introducing
stack-overflow paths. Tail-call diagnostics retain the current call location
and a bounded summary, rather than an unbounded logical call history.

Native host operations remain typed adapter calls using the same active budget.
An explicit core stack is insufficient if each recursive turn reenters through
a Rust host callback: integrate continuations where required or preserve a
separate bounded host reentry guard. Exercise a function that calls a native
projection while recursion is active. Failure reports include the failing
expression and a bounded chain of function call locations.

Functions receive their subject and data explicitly, with lexical locals and
no copying of the complete caller environment. Ordinary top-level ordered
bindings keep their existing rules. Proposed initial functions can access their
parameters, local bindings, named functions and registered pure host operations;
pass query parameters or outer report data explicitly. Effects remain visible
at the program/host boundary. Never memoize read-only host results merely because
they are read-only: surrounding host edits or context can change their meaning.

#### Values, collections and allocation

Move single-owner temporaries into their consumer. Borrow or share immutable
strings, constructed collections and native handles for repeated reads and
function arguments, rather than copying their whole contents. Choose borrowing
or reference counting where the actual lifetimes require it; retain owned
public results and explicit budgeted copies at boundaries that need ownership.
No JSON conversion is part of evaluation. Release dead frame values and loop
temporaries promptly; avoid a request-wide value arena that retains every
intermediate result until the request ends.

Preserve existing written evaluation order, eager comprehensions and eager
reductions. In particular, `forall([false, failing_expression])` still evaluates
the complete collection and fails. Conditions, Boolean short circuit, requirements
and defaults retain their specified branch laziness. Streaming/fusing a
comprehension into a reducer is deferred unless it preserves failure order,
checked-arithmetic behavior, host calls and documented budget accounting. A
separate lazy operation would require an explicit language decision.

For the new operations, target stable comparison sorting with O(n log n)
comparisons, one key extraction per row, and O(n) auxiliary entries. Use a
single scan for min/max, selecting the first tied item. Key extraction/type
failures and budget exhaustion must return located errors rather than being
hidden inside an infallible sorting comparator. Charge comparison work,
including string/key inspection, and admit scratch allocations. These are
algorithmic targets, not constant-cost promises for arbitrary keys.

String helpers inspect existing strings without constructing lowercase or
substring copies. Begin with exact, case-sensitive substring/prefix/suffix
semantics; propose true for an empty search string, without implicit Unicode
normalization or case folding. Keep traversal cost budgeted. Optional lookup
should reuse established record/key access, with an index added only when actual
repeated-lookup workloads justify its build and memory costs.

#### Budget and compatibility contract

Document the charges for preparation, calls, local reads, type admission,
collection construction, comparisons, host work, copies and rendering. Existing
work/results counters measure cumulative spending, not peak resident memory.
Charge newly allocated frames, temporary containers and copied bytes before
growth, and preserve the per-stream item ceiling. Releasing memory does not
refund cumulative spending. Sharing an already admitted value is distinct from
materializing another owned output; neither operation may bypass result limits.

Before changing existing copy/validation charges, specify the compatibility
impact on queries near their limits. Preserve defaults and existing entry-point
contracts until an explicit change is selected. Do not claim timing or memory
improvements by silently increasing allowances or skipping required checks.
The first implementation must demonstrate bounded scratch/frame allocation;
add a separate live-memory limit only if the existing allowance model cannot
express the needed bound simply. Host-internal allocation remains the host's
responsibility under its declared budget contract.

#### Two concrete function examples

The following is semantic pseudocode, not accepted syntax. Host primitive names
are proposed adapter contracts. Both examples lower to the same function/call
machinery and keep native arguments native.

Zirium: recursively total per-operation costs over direct containment children.
The adapter exposes `direct_children(Operation) -> sequence<Operation>` and
`cost_annotation(Operation) -> opt int`. A missing annotation is absent; a
malformed annotation is an error. Given an unannotated root and two leaf
operations with costs 3 and 4, the result is 8.

```text
fn subtree_cost(op: Operation) -> int {
    (cost_annotation(op) default 1)
        + sum([subtree_cost(child) for child in direct_children(op)])
}
```

Zincite: count calls with a chosen written name in a syntax subtree. The adapter
exposes `direct_children(Node) -> sequence<Node>`, `node_kind(Node) -> string`
and `written_call_name(Node) -> opt string`. The optional projection returns
absence when no direct written call name exists; it does not resolve or execute
a model function. A tree containing two calls written `sum` and one written
`max` returns 2 when the target is `sum`.

```text
fn count_calls(node: Node, target: string) -> int {
    let here = if node_kind(node) == "call"
                  and (written_call_name(node) default "") == target
               then 1 else 0;
    here + sum([count_calls(child, target)
                for child in direct_children(node)])
}
```

Both functions traverse containment, not arbitrary reference graphs. Repeated
reference paths would need an explicit visited-set or depth policy; recursion
does not implicitly deduplicate or replace the existing bounded path operation.
These examples make function overhead, native-handle passing, optional values,
integer overflow, eager reduction and depth limits observable without general
higher-order functions or a solver.

#### Measurement and delivery sequence

1. Record current parse/preflight/run/render costs for a small existing report,
   repeated field projection and repeated reads of a large constructed binding.
   Use representative inputs at a few increasing sizes. Attribute cost before
   selecting a storage rewrite; source inspection alone is insufficient.
2. Establish the host/runtime type and optional-value contract. Prototype
   prepared local slots and shared immutable arguments on one concrete path,
   measuring both small-query overhead and repeated evaluation costs.
3. Deliver named typed functions using the prepared path and explicit frames,
   with the two host examples. Include a branching recursive case so frame
   reuse alone cannot conceal exponential work; the shared work limit must
   bound it. Nontermination ends in an ordinary located limit error.
4. Deliver sorting/extrema and string helpers as separate focused slices. Keep
   all three original review suggestions visible; functions do not displace them.
5. Compare outputs and failure behavior, elapsed time by phase, available peak
   memory and temporary allocation/copy evidence. Include a wide report, bounded
   deep recursion, a repeated collection argument and stable sorting. Report
   limit refusals separately from completed throughput.

Use existing measurement drivers where practical and temporary instrumentation
for body-preparation, environment-copy and value-copy costs. Record revisions,
inputs, build profile, environment, repeated samples and spread. Set numerical
acceptance targets after collecting a baseline; do not invent a speedup target.
Acceptance must show that call setup does not rescan the body or deep-copy an
unchanged argument, local access uses resolved slots, and retained frame/value
memory follows live work rather than all completed calls. These are measurement
questions, not permanent tests of internal call counts. Retain focused public
behavior checks for runtime type errors, defaults, eager failure ordering,
recursive limits/cleanup, tie order and host identity.


### Selected contract for the next task bundle

The user requested tail recursion and a task split on 2026-10-14, then approved
the exact independently reviewed ten-task bundle. The following concrete choices
are the approved contract for that bundle. Ordinary existing programs retain
their accepted syntax, values and preflight guarantees.

- Optional values use `opt T`, `occurs(x)`, `absent(x)` and lazy `x default y`.
  Absence spells `<>` in Zincite and `null` in Zirium. There is one runtime
  absence state, distinct from unknown facts and errors. Arithmetic and ordering
  require present operands; optional equality compares absence with absence as
  equal and absence with a present value as unequal, subject to compatible
  declared underlying types. No absent-as-zero or absent-as-identity arithmetic.
- `lookup(value, key)` returns an optional field/cell value and `has_key(value,
  key)` reports membership, including a present cell containing absence. Use a
  string for a record label, an integer for a positional index and existing
  scalar/tuple keys for keyed arrays. Valid missing positions or sparse cells
  are absent; wrong key type/rank is an error. Missing fields of an untyped
  structural record are absent; a label outside a declared record schema is an
  error. Existing strict access stays strict. Declared optional record fields
  may be omitted; required fields may not. Output retains present absent fields
  distinctly from omitted fields/cells. Existing data-edit targets reject
  newly unsupported optional values before writing; expanding MiniZinc data
  serialization is outside this bundle.
- `default` checks runtime compatibility against available declared types;
  it never evaluates or scans the fallback on a present left value. Retain
  existing structural/preflight checks of all written branches. Bare absence
  can acquire its underlying type from a signature, collection context or
  fallback; absence by itself is not a universal escape from runtime checking.
- Host contracts describe scalars, optional values, positional sequences,
  tuples, records, sets, keyed arrays and nominal/native host types. Reuse
  current kinds and shape validation. Keep declaration constraints distinct
  from observed per-position `ValueType` facts. Typed interfaces check all
  elements; existing heterogeneous values remain legal outside incompatible
  typed interfaces. Native identity, enum resolution and model/MLIR type
  semantics stay with the host. No user-defined recursive data types are needed.
- Common function declarations use `fn name(arg: Type, ...) -> Type { body }`.
  A body has optional immutable local `let name = expression;` statements and
  one final expression. Expressions retain each host's natural syntax and
  pure native-query forms. Type spellings are `int`, `float`, `bool`, `string`,
  `opt T`, `sequence<T>`, `tuple<T, ...>`, `record {field: T, ...}`, `set<T>`,
  `keyed<T>` and registered host/nominal names. Keyed rank/axes remain runtime
  facts; this syntax adds no dependent types or implicit shape conversion.
- Function names are program-wide, declared before execution and available
  for direct and mutual recursion. Duplicate declarations and conflicts with
  host/builtin operation names are rejected. Parameters and locals use lexical
  scope without shadowing; functions receive outer program inputs explicitly.
  Signatures are mandatory, ordinary local types need no annotations. Existing
  external `param` declarations and CLI/library parameter transport remain
  `int`, `string` and `bool`; `opt` and native signature types do not implicitly
  expand those external inputs. Functions can return native values and use pure
  registered domain operations; they
  cannot emit or edit. No user overloads, function values, closures or imports.
- `sort(xs)` and `sort_by(rows, "field")` are ascending and stable; an optional
  `descending=true` reverses key ordering while retaining input order for ties.
  `min(xs)`/`max(xs)` return an optional scalar, and
  `min_by(rows, "field")`/`max_by(rows, "field")` return an optional original
  record. Empty input is absent and the first tied item wins. Keys must be
  present, homogeneous integers, finite floats, strings or members of one
  nominal type. Nominal order uses declared member order; string order is exact
  lexicographic order. Missing, absent or incompatible keys fail. A query can
  construct a key with `default` before ordering. No arbitrary comparators.
- `contains(text, fragment)`, `starts_with(text, prefix)` and
  `ends_with(text, suffix)` are exact, case-sensitive string operations with
  no normalization, coercion or regex. Empty search strings match. Non-string
  arguments fail; scans and diagnostic work share the request budget.

The new features require one compact public-behavior control for each changed
contract, including optional lookup/default failure distinctions, runtime type
refusals, native identities, direct/mutual and non-tail recursion, tail calls
beyond the active-depth ceiling, budget failure and cleanup, stable ties and
exact string matching. Reuse these cases across CLI/file/library paths rather
than multiplying a syntax matrix. Temporary instrumentation supports the stated
allocation and execution measurements; do not add timing or call-count tests.
Integration tasks may change the named consumer and Zirql adapter seams after
checking ownership and repository guidance. They retain separate consumer
commits and pinned Git dependencies. Publication and deployment are separate.


## Brainstorm: uses and further language directions

On 2026-10-14, the user requested a broader brainstorming session about uses,
language versus library features, conditionals and loops, explicitly allowing
language changes. The following candidates are discussion material. They do
not amend the approved tasks currently being implemented. Earlier exclusions
are open to reconsideration against concrete examples rather than permanent
limits on the language.

### Present control flow

The core has `ValueExpressionKind::Conditional`, with lazy selected-branch
execution and a mandatory final else. Zincite lowers MiniZinc conditional
expressions through its CST. The inspected shared value reader and Zirium's
textual supported-form list do not expose general conditional syntax; `if` in
Zirium comprehensions is a filter. Shared program statements currently bind or
evaluate expressions, with no general conditional statement or while/for loop
statement. Comprehensions already supply finite iteration. Typed recursive and
tail-recursive functions are approved work, not assumed completed behavior.

### Candidate uses

| Use | Example | Reusable language/library needs | Host responsibility |
| --- | --- | --- | --- |
| Interactive exploration | Find costly MLIR regions or inspect MiniZinc declarations | Concise pipelines, conditionals, projections, sorting | Domain navigation and names |
| Saved analysis reports | Summarize operation costs or model structure by module | Typed functions, grouping, aggregates, reusable query libraries | Native facts and output presentation |
| Model/data preparation | Derive keyed capacities and validate input relationships | Optional values, keyed transforms, joins/alignment, validation records | Model shapes and accepted edits |
| Structural linting | Report unwanted call shapes with source locations | Predicates, matching, report records, reusable rules | Syntax/semantic facts and source coordinates |
| Dependency analysis | Explain references or compute reachable dependents | Set/map operations, graph helpers, iteration to convergence | Edge meaning, native identity, incomplete information |
| Change analysis and transformation | Compare revisions or construct candidate rewrites | Correlation, grouping, functions returning change descriptions | Diff identity, validation and application semantics |
| Additional Rust hosts | Explore schemas, build graphs, logs or experiment results | Portable value functions and a small host integration contract | Loading data, external operations and domain meaning |

### Candidate feature ownership

Language candidates are expression-valued conditional blocks, lexical local
bindings, function calls, pattern matching and an explicit accumulation/loop
form if real programs need one. The evaluator must own their scope, branch
selection, evaluation order, return behavior and limits. A conventional
conditional statement becomes useful when selecting whole pipelines or host
emission/edit actions; that requires a deliberate effects decision beyond
pure conditional values.

Library candidates include enumerate/zip, fold/scan, distinct/grouping, joins,
record/map manipulation, numeric summaries, exact string operations and optional
regex/text helpers. Graph traversal, topological sorting and convergence helpers
are good shared candidates when their edge/identity contracts are explicit.
Loading files, model execution, source mutation and domain-specific facts remain
host integrations. Every candidate still needs its own empty-input, ordering,
failure and resource contract; this list is not an implementation checklist.

### Loops and higher-order operations

Iteration has three distinct user needs: constructing a collection, carrying
state through a known input, and repeating until a condition or convergence.
Comprehensions cover the first. A fold or scan can make the second clearer than
manual tail recursion. Explicit loop state can cover the third, with a returned
result and optional early exit. Tail-call frame reuse provides an execution
foundation but does not determine the most readable surface notation.

Compare a conventional local-mutation while loop with a loop that names its
state and supplies replacement state on each iteration. Choose using a concrete
worklist or running-summary example. Account for break/continue scope, eager
versus short-circuit consumption and failure behavior. Work limits bound loops;
algorithm-specific depth or iteration limits can additionally convey intent.
A convergence helper must define identity/equality, ordering and the outcome of
hitting its bound; budget exhaustion is not successful convergence.

Reconsider the approved first-release exclusion of function values narrowly:
passing a named pure function to fold, map or sort_by could provide substantial
reuse without requiring closures or a general callback system immediately.
Builtins can be generic before user functions need polymorphic inference.
Functions alone do not make a useful collection library ergonomic.

### Other design questions worth reopening

- Use one conditional construct returning values/blocks, exposed naturally in
  both frontends, before adding multiple forms with overlapping behavior.
- Destructuring records, tuples and optional values may simplify everyday code
  more than additional control keywords. Pattern matching is useful when hosts
  expose distinct native variants; those facts remain host-owned.
- Named result shapes and richer external parameter types make reusable queries
  easier to call. Current int/string/bool external parameters are a deliberate
  first-increment restriction, not a permanent design goal.
- A small import/library mechanism could make analysis rules reusable. Begin
  with host-supplied or local explicitly resolved libraries; choose name/version
  and loading behavior before adding packages or implicit file access.
- Eager values are simple, but large filters/reducers may justify an explicit
  iterator or generator form. Its short-circuit/failure behavior must be visible;
  it cannot silently replace existing eager array reduction semantics.
- Diagnostic records can carry severity, explanation and native subjects for
  lint/report workflows. Returning an expected failed check as data must remain
  distinct from runtime type errors or exhausted work.
- Functions returning candidate changes may compose better than editing during
  arbitrary loops. Hosts still validate and apply changes under their existing
  transaction contracts; the core must not imply automatic rollback.

The next discussion should use three small programs: a correlated analysis
report, a running/worklist computation and a validated transformation candidate.
Compare alternative syntax and identify which operations really belong in the
library. Keep everyday inline queries concise while allowing saved programs to
be readable. Do not infer a commitment to every candidate from the user's
permission to change the language.


### Exploration: patterns, control flow and reusable operations

The user endorsed exploring conditionals, named function arguments, fold/scan
and loops, and added destructuring assignments, destructuring function arguments
and match expressions. These are candidates beyond the approved bundle. The
user selected immutable destructuring bindings and explicit loop state.
Destructuring introduces fresh names; advancing loop state replaces the whole
state after evaluating its operands, without adding mutable local variables.

Use one shared pattern representation in binding, parameter and match positions.
Start with names, `_`, tuples, records with explicit field selection/renaming,
and optional presence/absence; literals are useful in match arms. Host-declared
nominal cases can follow when a concrete domain exposes them. An optional
presence pattern is a view of an existing optional value, not a new runtime
wrapper or a second absence state. Destructuring a native object requires an
explicit host projection or declared view; Rust implementation fields are not
implicitly accessible to the language.

Proposed binding behavior:

- Evaluate the right-hand side once. Match all required fields/positions before
  installing names; a mismatch returns a located error with no partial binding.
- Tuple length is exact. Record patterns name required fields, rename with
  `field: local`, and use `..` to explicitly ignore extra fields. `_` discards.
  Avoid an allocating rest-copy binding until a concrete case needs it.
- Bindings stay lexical; duplicate names and forbidden shadowing remain errors.
  Parameter patterns use the same rules after argument admission and before
  the body runs. Annotate the complete parameter's type; inner names get their
  runtime contracts from its fields/positions.
- Presence/variant patterns that might not match belong naturally in match
  expressions. If allowed in ordinary let/parameters, failure is an error,
  never implicit filtering. Comprehension pattern mismatch likewise needs an
  explicit filtering form before it can skip candidates.

Proposed value matching evaluates its subject once, checks arms in written
order, and evaluates only the first successful arm. Arm-local bindings are
available to an optional Boolean guard and result. A false guard tries the
next arm; a guard error or Unknown does not. Require a final wildcard initially
for a guaranteed fallback, rather than building general exhaustiveness
inference. Matching does not catch evaluation failures. Optional lookup has
already normalized absence; use has_key separately when membership versus a
present absent value matters.

Use `match (value) { pattern => expression, ... }` as a candidate spelling to
distinguish it from existing native `match { from ...; bind ...; where ...; }`.
The native form enumerates captures; the value form selects one result. Both
may use native values without sharing enumeration semantics. Each selected
arm/block and conditional result can be a tail position; pattern/guard work
must finish before frame reuse, with no retained continuation solely for a
completed match.

Example syntax sketches, not accepted grammar:

```text
let (label, count) = summary;
let {name, cost: estimated_cost, ..} = row;

fn add_cost(total: int, (_, cost): tuple<string, int>) -> int {
    total + cost
}

match (lookup(row, "cost")) {
    absent => 0,
    present(cost) if cost >= 0 => cost,
    _ => require(false, 0, "negative cost")
}
```

Expose conditional expressions consistently in both frontends, with blocks
containing lexical bindings and a final value. Retain selected-branch evaluation
and existing preflight checks. Conditionals selecting edits/emissions remain a
separate effects decision. Pattern names/syntax may adapt to a frontend, but
match order, binding failure and guard semantics must be shared.

Permit named pure function references as candidate arguments to closed shared
library operations. Resolve the function and its arity/purity before iteration,
including empty input; validate actual arguments/results at runtime. Proposed
`fold(items, initial, step)` applies `step(state, item)` left-to-right and returns
initial for empty input. Proposed `scan` uses the same order and returns each
post-step state, excluding the initial state: n items yield n results and empty
input yields an empty sequence. Input collections remain eager; neither form
implicitly short-circuits or hides failures. Passing named functions can precede
closures, anonymous functions or user-defined polymorphism.

For condition-driven iteration, use the selected explicit loop state and test
its readability with a real worklist/accumulation example. A syntax candidate has
`loop (index = 1, total = 0) { ... }`, `break result` and
`continue (index + 1, total + cost)`. Evaluate every next-state operand against
the old state before installing any new value; retain compatible state contracts
and lexical scope. Ordinary local bindings remain immutable in this option.
Each iteration charges work, releases dead state and preserves host/native
identity. Nested break/continue target the innermost loop initially. Loop exit
and function return are separate operations. Final loop syntax is undecided.

A loop and fold may reuse the tail-call/continuation machinery; this does not
require expanding user code into a fresh function or rebuilding bindings per
iteration. Borrow/share destructured fields, resolve patterns to local slots,
and charge inspection and allocations. Failed arms and guards spend work too;
checking a pattern must not deep-copy a large record or collection merely to
bind one field. Returned/retained scan states follow the existing ownership and
result-budget contract.

Language owns patterns, scopes, conditional/match selection and loop control.
The shared library owns fold/scan and named-function-based collection helpers.
Hosts supply native projections, declared variants and operations. Before tasks,
compare complete programs for report accumulation, optional-value classification
and a graph worklist, and decide pattern syntax and matching coverage. Immutable
bindings and explicit loop state are settled for this exploration.


### Worked example: declaration reachability with a worklist

The user requested a complete example to exercise the proposed patterns and
explicit loop state. This is a design sketch; the new syntax and collection
helpers are not implemented or added to the current task bundle.

Question: can a starting declaration reach a target through zero or more
reference edges? The graph has ordered adjacency A -> [B, C], B -> [D],
C -> [B], D -> [A], E -> []. Thus there is a cycle and two routes to B;
E is a real, disconnected declaration. A reaches D; A does not reach E.
A declaration reaches itself through zero edges.

The example host provides native Declaration handles, same_declaration(a, b)
and reference_info(d), an explicit record view containing ordered native
`targets`. For this example, reference_info returns a complete adjacency list
or a located error; incomplete semantic knowledge cannot become an empty list
or a false reachability result. A production adapter must explicitly implement
that contract. These are proposed function bindings over domain facts, not
claims about existing Zincite textual function names.

```text
fn reaches((start, target): tuple<Declaration, Declaration>) -> bool {
    loop (pending = queue([start]), seen = identity_set<Declaration>()) {
        match (pop_front(pending)) {
            present((current, remaining)) => {
                if current in seen {
                    continue (remaining, seen);
                } else if same_declaration(current, target) {
                    break true;
                } else {
                    let {targets, ..} = reference_info(current);
                    continue (
                        push_back_all(remaining, targets),
                        insert(seen, current)
                    );
                }
            },
            _ => break false
        }
    }
}

reaches((A, D))  // true
reaches((A, E))  // false
```

A through E above denote supplied native declarations, not string names that
implicitly resolve themselves. The function has one destructured tuple
parameter. pop_front returns an optional (item, remaining_queue) pair; matching
binds those names without a sequence-tail copy. The record pattern explicitly
projects targets and ignores other declared view fields. Each continue computes
both successor-state operands against the old bindings before replacing state.
All branches either continue or break; loop exits return the function result.

Trace when the target is E:

| Taken item | Action | Pending after the step |
| --- | --- | --- |
| A | Expand | B, C |
| B | Expand | C, D |
| C | Expand | D, B |
| D | Expand | B, A |
| B | Already seen; skip | A |
| A | Already seen; skip | empty |
| none | Break with false | empty |

For target D, the fourth step breaks with true. A small temporary Python deque
model reproduced these two traces; it validates the example algorithm, not
Zirql parser acceptance or implementation performance.

Required library contracts are FIFO queue construction, optional pop_front,
push_back_all retaining edge order, an identity set, membership and idempotent
insertion. Native membership uses the host's semantic identity, including its
context/side distinctions where relevant, never a printed name. This identity
set is an explicit new library capability: existing sorted scalar/nominal sets
do not already provide native-handle membership, and their ordering contract
must not change incidentally. A visited set need not expose iteration order.

The visited set is updated on expansion. Each reachable node is expanded at
most once, although duplicates may enter the queue. With expected constant-time
identity membership, work is O(V + E) plus host adjacency discovery; pending
entries can occupy O(E) space and visited identities O(V). All queue growth,
identity operations, pattern work and host scans share the active budget.
Moving uniquely owned queue/set storage can make transitions efficient while
preserving immutable language semantics. Retained aliases must preserve their
old values; a fresh full queue/set copy on every step would destroy the intended
complexity and needs to be caught by measurements.

This example suggests retaining simple patterns rather than requiring list
rest patterns: pop_front exposes the needed decomposition directly. It also
shows a limitation of positional continue arguments as state grows; named
state updates are a later ergonomic choice, not necessary for this two-field
loop. The loop makes early exit and repeated state transitions explicit.
General reachability itself could be a library helper; existing bounded paths
remain appropriate when a caller wants an explanatory witness. The value of
this example is the ability to write a custom worklist rule without a new
host-specific evaluator or callback implementation.


### Selected contract for the patterns and worklists task bundle

The user endorsed the worked example and requested tasks on 2026-10-14. The
following choices turn the exploration into an exact reviewable bundle. They
extend base-031 through base-038 after the relevant prerequisites; their
completion does not become a new condition for those existing tasks.

- Use one located pattern representation for immutable let bindings, function
  parameters and value match arms. Support names, `_`, nested fixed-length
  tuples, records, literal scalar patterns, `present(pattern)` and `absent`.
  Record fields support shorthand and `field: pattern`; identifiers or quoted
  static labels name fields. Without `..`, the record shape must match exactly;
  final `..` ignores extra fields without copying them. No bound rest value,
  sequence-rest pattern, arbitrary extraction call or user-defined variant yet.
- Evaluate binding values and function arguments once in written order, admit
  function arguments against their whole declared parameter types, then match
  patterns. Install names only after the whole binding/parameter set succeeds.
  A binding/parameter mismatch fails at the pattern; it never filters input.
  Reject duplicate bindings and prohibited shadowing before execution, even
  in unused arms. Native records use explicit host views/captures, retaining
  handles and coordinates rather than reflecting Rust object fields.
- `match (subject) { pattern [if guard] => expression, ... }` is an expression
  distinct from existing `match { from ... }` capture enumeration. Evaluate the
  subject once and select the first matching pattern with a true guard. Shape,
  literal and presence mismatches try the next arm; host/type-admission and
  resource errors propagate. Guards require known Booleans. False continues;
  Unknown/non-Boolean/error fails. Require one final unguarded `_` arm initially.
  Bindings are arm-local and only its selected body runs. Matching is not error
  recovery or an implicit optional conversion.
- Expression blocks appear in function bodies, conditional branches, match
  arms and loops, containing local let bindings and one final expression or
  loop transfer. Ordinary record literals retain their established grammar;
  blocks are recognized in these explicit control positions. The common
  control spelling is `if condition { ... } else { ... }`, with else-if chains
  and a final else. Zincite also retains its existing MiniZinc conditional
  expression spelling and expressions inside blocks; both lower to the same
  shared conditional semantics. Reuse general conditionals already delivered
  for base-034/base-035 instead of reimplementing them.
- Blocks, guards, loops and called functions are pure. Top-level existing
  emission/edit statements remain outside them. Selected final conditional and
  match expressions preserve compatible tail calls; finished pattern/guard
  work cannot leave an accumulating continuation behind.
- `loop (name = initial, ...) { body }` introduces fresh immutable state names.
  Initializers evaluate in the outer scope before any state names are installed.
  `continue (next, ...)` evaluates values left-to-right against the old state,
  checks the established state contracts, then replaces all state together.
  Arity must match and every state is supplied. `break expression` is the loop
  result; nested transfers target the innermost loop. Transfers outside a loop
  fail preflight and cannot escape a function boundary. Each body path must end
  in break/continue (possibly through nested if/match blocks); fallthrough is a
  preparation error. No implicit unit value, mutable locals, loop labels or
  named state updates. Validate declared result contracts on selected exits.
- Loop iterations consume work without growing active call depth. Expression
  nesting, non-tail recursion and host reentry retain their guards. State slots
  use runtime contracts inferred/admitted at initialization or supplied by
  context; this adds no general static inference. Admission of empty native
  collections needs an explicit element contract. Releases and error unwinding
  preserve aliases and avoid unbounded stack/history/retained-temporary growth.
- `fold(items, initial, step)` and `scan(items, initial, step)` accept a resolved
  named pure function in the step position. Resolve signatures/arity/purity
  before iteration, even for empty input; pass state and item with the existing
  runtime contracts. Enforce a step result compatible with the state contract.
  Evaluate items then initial once and invoke step left-to-right once per item.
  Input is an eager positional sequence, possibly containing native values;
  keyed arrays require explicit conversion. Fold of empty input is initial;
  scan retains each post-step state, excludes initial and is empty on empty
  input. Retained earlier scan states never change when later state advances.
  Runtime step errors propagate and retain locations/call context. This is a
  narrow extension to base-033's exclusion of function values: no stored or
  returned function values, closures, lambdas, user generic functions or new
  function-selector overloads for sorting are included.
- Add library runtime types `queue<T>` and `identity_set<T>` for native T, with
  native identity supplied by the host for the set. Queue accepts admitted
  values including native handles. `queue(items)` infers from nonempty admitted
  input or context; `queue<T>(items)` explicitly supports empty input.
  `pop_front(q)` returns `opt tuple<T, queue<T>>`; empty is absent.
  `push_back_all(q, items)` preserves existing queue then item order.
  `identity_set<T>()`, `item in seen` and `insert(seen, item)` supply empty set,
  membership and idempotent insertion. Host identity tokens/equality/hash must
  agree and retain document/diff-side distinctions; stale/foreign handles fail
  under existing host policy. Set iteration order and general set algebra are
  outside this slice. Existing ordered scalar/nominal sets retain their contract.
- Queues and identity sets are internal computational values, admissible in
  function signatures, locals and loop state. Their direct rendering or data
  serialization is unsupported and yields a located error; no debug-string
  fallback. Public native results can retain them for continued library use.
  Runtime type admission covers nested queue results without reconstructing or
  rescanning an unchanged whole queue on every pop. The queue/set operations
  must preserve retained aliases while allowing efficient unique-owner updates.
- The Zincite integration supplies the example's Declaration identity and an
  explicit reference_info view. Reuse the existing referenced_declarations
  edge order/meaning, with complete adjacency or a located refusal. The Zirium
  version uses Operation identity and direct SSA users, with a correspondingly
  complete-or-error view. Same-operation/declaration checks must agree with
  identity-set membership. Functions return Boolean reachability, including a
  zero-edge path; no path witness, extra graph algorithm or solver is added.

The shared-library tasks need callable, independently demonstrable core paths;
consumer tasks add the textual and typed-builder/library paths in each host.
The patterns/match integration precedes the loop/reducer/worklist integration
within a host. Preserve existing short queries, native capture match syntax,
preflight validation, output buffering and pinned Git dependencies. No changes
to data-edit policies, package publication or site deployment are included.

Validation uses a compact destructured report, an optional guarded match, an
accumulator fold/scan, and the worked graph with cycle/duplicate/disconnected
nodes. Add focused refusals for failed destructuring, bad guards, invalid loop
transfers/state, non-pure/wrong-signature steps and exhausted budgets; reuse
these cases across hosts. Use temporary measurements of a retained alias/scan
state and increasing wide/cyclic worklists to check copying and allocation.
No timing assertions, internal call-count tests or corpus campaign. Run each
changed repository's required checks and record separate consumer commits.
