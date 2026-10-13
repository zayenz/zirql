# Research into Zirql language extensions

This research asks which extensions would make Zirql more useful and enjoyable
for native analysis, reports and data construction. It combines four research
reviews with inspection of the shared evaluator and both consumers on
2026-10-13. These are design recommendations, not implemented features or an
approved task bundle. The [base brief](../brief.md) owns shared decisions.

The user's clarified direction is a substantial common language whose consumers
mainly adapt it to their domains. Accordingly, the recommendations below place
general language behavior in Zirql. An existing host hook can help establish
feasibility, but it is not a reason to leave reusable semantics in each adapter.

Read [the example programs](language-design-examples.md) for complete proposed
queries, inputs, expected results and unresolved design choices.

## Existing foundations

The inspected Zirql core already supports native programs, immutable bindings,
located expressions, checked scalar operations, positional comprehensions,
nominal members and sparse multidimensional keyed arrays. Its common collection
stages include count, stable unique and string tally. Null and string-keyed maps
are mentioned in the brief but are not current `ConstructedValue` variants.

Zirium already has sorting, extrema, `map_by`, cardinality checks, fixed points,
Markdown reports and semantic diff queries. These are not missing capabilities.
Zincite has positional comprehension reports; its keyed report integration and
atomic data construction remain existing planned work.

Inspection points, relative to the sibling repositories:

- Zirql [`src/program.rs`](../../../src/program.rs): native record fields,
  initial bindings and nested host evaluation.
- Zirql [`src/value.rs`](../../../src/value.rs): supported value and
  expression variants, key access and generator mechanics.
- Zirium `docs/query-language.md`: per-item `map_by`, literal restrictions,
  comprehensions, checks, traversal and diff behavior.
- Zirium `docs/query-dsl.md`: native builders and caller-side report construction.
- Zincite `docs/reference.md` and `docs/query-comprehension-contract.md`:
  supported expressions, completeness, literal projection and data boundaries.

The inspected revisions were Zirql `3a63bf53cd4aff7a70d611c4d150a614e86968e6`,
Zirium `d7869d3b8b74a727bdd2f8a03865724b7ffe3b90`, and Zincite
`12b821fb942d1f1b1e64a1b54f5471045102d768`. Recheck live support before planning
implementation. Proposed syntax has not been executed or benchmarked.

## Recommended capabilities

| Capability | Concrete benefit | Base responsibility | Domain adaptation |
| --- | --- | --- | --- |
| Per-item record projection | Correlated, multi-field reports without unique map keys | Subject binding, field evaluation, cardinality and ordered row construction | Attribute/name/type projection and navigation |
| Fixed reducers | Totals, counts and finite Boolean invariants | Checked reduction and empty-input rules | Surface spelling |
| Typed parameters | Reuse query files with caller-supplied data | Declaration checking, admission and binding | CLI transport and native domain types |
| Grouping | Summaries retaining all rows with repeated keys | Typed key comparison and stable groups | Any domain-specific identity operation |
| Value requirements | Explain invalid input or constructed data | Pure conditional requirement and located failure | Completeness and report/exit policy |
| Keyed entry access | Work directly with sparse or multidimensional cells | Entry order, key types and explicit alignment | MiniZinc target shape and serialization |
| Native captures | Retain related subjects in one match | Binding scope, dependent enumeration and guards | CST/MLIR operations and semantic identity |
| Explanatory paths | Show a connection through known domain edges | Bounded traversal semantics if selected as a base feature | Edge meaning, native identity and presentation |

### Per-item record projection

A function report should express its name, operation count, selected-operation
count and operation kinds together. `map_by` requires a unique string key;
separate global projections can lose their per-item relationship. Shared
projection should evaluate each field against one bound native subject and
produce one ordered record per input item. Duplicate labels within a record
fail; duplicate row values or function names remain ordinary data.

[Nushell's `each`](https://www.nushell.sh/commands/docs/each.html) demonstrates
the usefulness of per-row transformations. Its default omission of null
results should not become an implicit Zirql cardinality rule. Define explicit
filtering separately from projection.

The existing native record representation and nested evaluation make this
plausible without a universal object model. The intended owner is the shared
language: hosts provide primitive projections, while Zirql owns the iteration,
subject scope, purity checks and report construction. Native handles remain
distinct from constructed scalars.

### Fixed reducers and quantifiers

Start with `length`, checked integer `sum`, and fixed-Boolean `forall`/`exists`.
They answer total-demand and report-invariant questions using values that
comprehensions already construct. The frontend may choose `all`/`any` spellings
with the same semantics.

[MiniZinc aggregation](https://docs.minizinc.dev/en/2.10.1/modelling2.html#aggregation-functions)
supplies useful empty-input conventions: sum is zero, universal quantification
is true and existential quantification is false. Later extrema can reject
empty input. Numeric reduction follows declared iteration order and checked
arithmetic; floating accumulation needs an explicit numerical contract.

Initially reduce materialized arrays. Construction and validation complete
before reduction, so `forall([false, failing_expression])` still encounters the
construction failure. Direct generator reduction could skip later heads and
their failures; that is a separate semantic choice, not an invisible optimizer
change. A closed aggregate operation is enough; arbitrary fold callbacks are
not required.

### Typed inputs and reusable interfaces

BAML makes typed inputs and results part of a readable function interface.
CEL checks expressions against a host-declared environment. Together these
suggest typed query parameters and, when useful, declared report shapes.
See [BAML functions](https://docs.boundaryml.com/ref/baml/function),
[BAML classes](https://docs.boundaryml.com/ref/baml/class), and
[CEL environments](https://cel.dev/overview/cel-overview).

Initial bindings already exist. Zirql should own parameter declarations,
missing/extra/type checks and admission into those bindings. Hosts transport
typed values from CLI/library callers and resolve domain-specific types.
Parameters used in predicate arguments stay values rather than substituted
query source. This makes saved queries reusable without a function runtime.

Named structural report shapes are useful when actual reports repeat a shape.
Declared type constraints need not be the same representation as current
`ValueType` facts, which also encode observed positions and deferred knowledge.
Preserve native types, nominal identity and keyed axes. Internal named queries,
imports and a broader function system remain design options rather than an
automatic consequence of parameter support.

### Stable grouping of records

`group_by(rows, "machine")` could return ordinary records with `key` and `items`
fields. Existing comprehensions and reducers then form per-machine totals or
grouped violation reports. [PRQL grouping](https://prql-lang.org/book/reference/stdlib/transforms/group.html)
shows how grouping composes with further transformations.

Prefer first-seen group order and original member order. Typed key identity
prevents integer, string and nominal keys from collapsing through spelling.
Missing fields and incompatible key types fail. A static field selector is a
useful first form; users can project a derived field before grouping. Grouping
belongs in the base and becomes more useful after correlated projection exists.

### Requirements and diagnostic values

[BAML checks and assertions](https://docs.boundaryml.com/guide/baml-advanced/checks-and-asserts)
distinguish advisory validation from strict failure. Borrow the distinction;
keep Zirql's report cardinality and failure rules explicit. Failed assertions
must not silently remove constructed members or keyed cells.

A pure `require(condition, value, message)` can validate a relationship before
evaluating the requested value. Static checks cover every operand; runtime
evaluation chooses the value only on true. Ordinary records and comprehensions
already express advisory results and witnesses. A new diagnostic wrapper on
every value is unnecessary.

Zirium's existing `check(n)` handles cardinality. This proposal addresses value
relationships. Host incompleteness remains separate from false: an incomplete
selection cannot prove that no violations exist. Type, arithmetic and resource
errors remain request failures. Requirements do not add rollback to live edits.

### Keyed entries and deliberate alignment

An `entries(a)` operation would expose present key/value pairs in established
order, including sparse rank-greater-than-one arrays. Current rank-one
`index_set` does not cover that use. Entry access should retain typed keys;
the original array continues to carry axes, including empty axes.

Exact alignment could later pair arrays only when their axes and actual cell
keys agree. Equal length is insufficient. [Xarray's exact alignment](https://docs.xarray.dev/en/stable/generated/xarray.align.html)
is a useful reference for explicit refusal on incompatible indexes. Automatic
intersection, filling and reindexing would need separate visible operations.

Finish the planned keyed consumer path before adding these conveniences.
Generic entry and alignment operations belong in Zirql even if Zincite is the
first consumer. Rectangularity required by a MiniZinc target remains host policy.

### Native structural captures

Named captures can express a relationship while retaining all subjects for
later projection. Useful cases include an MLIR operation using the same SSA
value twice, or a MiniZinc call with an argument referring to a proven
declaration. [Tree-sitter captures](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html)
and [ast-grep patterns](https://ast-grep.github.io/guide/pattern-syntax.html)
demonstrate the expressive value of structural patterns.

Zirql should own capture scope, finite dependent enumeration, guards and native
match-record construction. Hosts supply the structural relations and equality
operations. Matching can share generator mechanics with comprehensions while
keeping native handles out of scalar arithmetic. Decide through examples
whether it needs a separate surface construct.

Start with required single-value captures and explicit comparisons. Same
spelling, structural equality, same declaration and same SSA value are different
questions. Charge candidate combinations before guards. Variadic holes and
unordered matching add substantial search behavior; defer them until a concrete
pattern needs them.

### Explanatory paths

Existing reachability answers which objects are connected. A deterministic
shortest witness could show how a selected operation reaches a return through
SSA uses, or how declarations connect through proven written references.
[CodeQL path queries](https://codeql.github.com/docs/writing-codeql-queries/creating-path-queries/)
demonstrate results with endpoints and connecting steps.

If selected, generic bounded traversal, ordering and path construction belong
in the base. Hosts define edges, identity and limitations. A path through known
edges is a witness; no path within a bound does not prove global disconnection.
All-path enumeration and general path-expression syntax can wait.

## Scope and priority

Develop correlated projection, reducers and typed parameters first. Grouping
and requirements compose with them. Explore native captures against the third
example before choosing syntax. Keyed entry helpers follow the existing keyed
integration; explanatory paths need one concrete investigation use case.

Small generator destructuring or narrow missing-field/key defaults may earn
their place later. Defaults should handle actual absence only; a general
`try(expression, default)` could conceal arithmetic failures or exhausted work.

Full relational joins/windows, recursive rules, constraint unification,
arbitrary callbacks and automatic rewrite systems introduce additional
semantics not required by these examples. Soufflé's
[Datalog tutorial](https://souffle-lang.github.io/tutorial) is a useful future
reference for relational analysis, but existing finite generators and graph
operations should be assessed first. This is sequencing advice, not a decision
that Zirql must remain a minimal execution kernel.

Confidence is high in the inspected capability distinctions and ownership
opportunities. Priority is a design judgment. The examples are intended to make
that judgment discussable; there is no performance evidence or parser acceptance
claim for the proposed forms.
