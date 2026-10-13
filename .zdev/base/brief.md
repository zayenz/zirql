# Shared query language foundations

## Objective

Build Zirql as a Rust framework for purpose-built query languages in Zirium
and Zincite. Provide a substantial shared base language, including query
execution and bounded comprehension evaluation. Consumers primarily adapt that
language to their domains through native types, operations and surface syntax.

Zincite is about to add programs, bindings and comprehensions. Zirium already
implements programs and structured reports. A common owner should prevent
parallel implementations of those language rules and preserve the useful
differences between the tools.

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

### Proposed extension bundle

The task bundle selects the examples' explicit `project(subject, {fields})`,
required `param name: type` declarations with `int`, `string` or `bool` types,
eager array reducers and conditional `require`. Initially `require` accepts a
literal string message; it preflights all operands and evaluates its value only
after a true condition. It includes shared `match` clauses (`from`, `bind`,
`where`) returning native capture records. Named result schemas, parameter
defaults, arbitrary functions and lazy generator reduction remain deferred.
The user approved this exact bundle on 2026-10-13. Tasks base-018 through
base-025 record its implementation scope; the language forms below remain
unimplemented until their tasks are completed.

Use Zirium's existing query argument for inline programs. Add `--program TEXT`
to both Zincite query commands, mutually exclusive with `--program-file` and
the legacy positional query. Both hosts accept repeated `--param name=value`
for declared inputs: strings are passed verbatim, integers are checked decimal
`i64`, and Booleans are exactly `true` or `false`. Duplicate, missing, extra or
incompatible parameters fail before domain execution. Library callers supply
typed values. Files remain useful for longer programs; single expressions and
pipelines keep their existing host emission conventions.

The remaining selected forms are `group_by(rows, "field")` with first-seen
groups and typed keys; `entries(a)` for keyed cells; `zip_exact(a, b)` for
keyed arrays with identical ordered axes and present keys, including identical
sparse coverage, producing two-element tuple cells; and one deterministic
shortest path per reachable source/sink pair within an explicit edge-depth
bound. Hosts supply path edges and native identity. Their uncertainty and diff
side rules survive shared execution. Path queries are read-only and do not
enumerate all possible paths.

Path steps include both endpoints. Zirium's `users` edge means one operation
to one direct SSA user operation. Zincite's new `referenced_declarations` edge
means one declaration to a proven target of a reference written anywhere in
its owning region, following `transitive_references`' existing region and
reference-order rules. Each such declaration-to-declaration connection counts
as one edge; reference occurrences are not additional steps. Repeated targets
are considered in first-reference order. Unknown target predicates cause a
located refusal, and unresolved edges retain the host's incompleteness.

For the MLIR fixture in the examples, this proposed command has one witness
with steps `[arith.muli, arith.addi, func.return]` in `mix`:

```sh
zirium 'filter(op("arith.muli")) | paths(users, to=op("func.return"), max_depth=2) | json' input.mlir
```

For a model containing `int: c = 1; int: b = c; int: a = b; solve satisfy;`,
this proposed command has one native declaration witness `[a, b, c]`. At depth
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
required for Zincite. The proposed Zirium surface extends its JSON-shaped
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

Zincite currently has unrelated in-progress semantic-owner edits. Consume its
public APIs and preserve that work. Before implementing its integrations,
reconcile Zincite base-126 (read-only programs), base-127 (data construction) and
base-128 (measurement) with this plan so their outcomes do not get implemented
twice. Do not mark a consumer task done merely because Zirql has an equivalent
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
these boundaries. Zincite's data-construction grammar, operation subset,
target/key/empty-axis and comment policies retain their existing attended gate.
The repository is public and the crate name is reserved. Before another
publication or consumer distribution, follow the development and release
policy in User decisions.

The approved extension bundle records the selected concrete language choices.
Result schemas and broader function syntax remain future
design work. Shared base ownership and short command-line usage are settled.
