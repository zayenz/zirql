# Adapter examples

Zincite and Zirium apply the same language to two different data models. Zincite
exposes MiniZinc source and model facts; Zirium exposes MLIR operations and
their relationships. Both need to keep the objects attached to their source
while a query selects, navigates and groups them. This page explains those
implementation choices. Use the [Zincite site](https://zayenz.se/zincite/) and
[Zirium site](https://zayenz.se/zirium/) for commands, accepted syntax and
workflows.

For tabular data, start with the [CSV host tutorial](csv-host.md). It uses
constructed arrays and records, with headers declaring each row's fields.
The [CSV queries](csv.md) show how that representation supports filtering and
reports across files. The examples below explain when an adapter needs native
objects and retained source identity.

## Choose native values

| Boundary | Word example | Zincite | Zirium |
| --- | --- | --- | --- |
| Input | Array of strings | Source nodes, declarations and references | Document-owned operation handles |
| Identity | Array position identifies an occurrence; string equality groups spellings | Source/model identity, including scope and file | Operation and SSA-value identity within a document |
| Domain operations | None for constructed word analysis | Syntax selection, declaration resolution and uses | Regions, operands and dataflow users |
| Projection | Strings are already values | Names and model facts become report values | Names and operation properties become report values |
| Retained owner | Constructed array | Source files and model context | Parsed document |

The two occurrences of `apple` belong to one spelling group but retain their
separate array positions. Two source declarations with the same spelling may
identify different objects. Two MLIR operations with the same name are also
distinct. The host preserves that distinction until a query explicitly projects
names and groups them.

## Supply facts and uncertainty

Zincite resolves source declarations and references in a retained model context.
An include or builtin context can be missing, leaving some facts unknown.
Its adapter preserves those limitations beside the result. Negating an unknown
predicate does not make the fact known.

Zirium supplies operation structure and SSA relationships. A value's identity
includes its document and result or block-argument position. A defining
operation alone cannot distinguish two results of that operation. Printed SSA
names also do not establish identity across scopes.

Zirql combines predicates, sequences operations and retains native values.
Each adapter supplies the facts and knows when they are incomplete.

## Project and group

A native query can select objects, project a name into a record and group those
records by name. The grouping step has the same semantics as the
[grouped-occurrence example](examples.md#keep-the-occurrences-in-each-group):
first-seen groups, ordered members and retained duplicates.

For Zincite, the projection reads a declaration or source node. For Zirium, it
reads an operation. The shared evaluator can then count group members and sort
the report. It needs no MiniZinc or MLIR-specific grouping rules.

An adapter implements
[`ProjectionHost`](host-integration.md#correlated-native-projection) to connect
native field evaluation to constructed results. Native capture matching can
retain several related handles before that projection.

## Define path edges

A path adapter supplies ordered neighbours and native identity. Zirql supplies
bounded traversal and selects one shortest witness for each reachable
source/target pair within the edge bound.

Zirium's `users` edge connects an operation to a direct SSA user operation.
Zincite's `referenced_declarations` edge connects a declaration to a proven
declaration target referenced in its owning region. A hypothetical word host's
`next_word` edge could connect adjacent occurrences.

The same path evaluator works with each choice. The host defines what one edge
means and retains source coordinates and uncertainty. A missing witness within
a depth bound does not establish that no longer path exists. See
[the path adapter](host-integration.md#native-path-adapter).

## Keep domain policy with the application

Zincite's nominal members get their identity from its retained model. Its data
construction also enforces the model's axis and shape rules. A sparse keyed
array can be valid in Zirql but invalid for a particular MiniZinc value.

Both applications own rendering and source-edit policy. A native result can
be rendered as text, a report or a candidate edit, depending on the host.
Zirql's evaluator does not establish dialect validity or make a source change
safe to commit.

When designing another adapter, make these choices explicit: what identifies an
item, which operations expose facts, which facts may be unknown, and how results
leave the evaluator. The [implementation reference](implementation-reference.md)
connects those choices to the Rust interfaces.
