# Semantic model integrations

The [word examples](examples.md) need one domain predicate: a prefix test on a
borrowed string. A larger host supplies different native items and operations,
while using the same language evaluator. Zincite and Zirium are two such hosts.

| Choice | Word host | Zincite | Zirium |
| --- | --- | --- | --- |
| Native items | Borrowed words | Source nodes, declarations and references | Document-owned operation handles |
| Predicate meaning | Exact text prefix | Written kind or resolved model fact | Operation name, dialect or attribute |
| Navigation | None in the basic example | Source structure and declaration uses | Regions, operands and dataflow users |
| Retained owner | Source string | Source files and model context | Parsed document |
| Output | Caller inspects the native result | Source rendering or an inspection envelope | Selected fragments, JSON or native results |

The host parses its domain syntax and lowers shared constructs to Zirql's
located Rust representation. Zirql executes those constructs with the active
budget. Native operations remain host hooks: the core has no dependency on
either consumer crate and supplies no MiniZinc or MLIR semantics.

## Zincite: source and model facts

Zincite starts with source-preserving selections. A syntax predicate can ask
whether a selected node is a written constraint. Save this as `model.mzn`:

```text
int: capacity = 20;
constraint capacity > 0;
solve satisfy;
```

With Zincite installed, run:

```sh
zincite query 'filter(kind("constraint")) | count' model.mzn
```

The command prints `1`. Zincite parses the file, selects its written items and
implements `kind`. Zirql combines predicate answers and counts the retained
selection. The host renders the count as a decimal integer. Syntax inspection
does not need a MiniZinc compiler, solver or standard library.

A semantic query needs more than node text. Zincite resolves declarations and
references using a retained model context, and distinguishes the same name in
different scopes or files. Its source results retain file identity and byte
ranges. If an include or builtin context is missing, some facts remain unknown.
Semantic reports keep `complete` and located `limitations`; available results
alone do not establish completeness. Negating an unknown predicate does not
make it known.

Zincite also owns the supported MiniZinc value subset. Nominal members get
their identities from the model. Keyed values must meet the host's axis and
shape rules where those rules apply. A generic sparse keyed array can be valid
in Zirql and still be refused by Zincite. Source edits have separate target,
comment and atomic-write requirements.

See [Zincite source queries](https://zayenz.se/zincite/guides/queries/) for syntax
inspection and semantic context requirements.

## Zirium: operations in a retained document

Zirium's native items are operation handles. A handle identifies an operation
in its document; its printed name is a property of that operation. Save this
as `operations.mlir`:

```text
func.func @mix(%x: i32, %y: i32) -> i32 {
  %a = arith.addi %x, %y : i32
  %b = "arith.muli"(%a, %y) : (i32, i32) -> i32
  %c = arith.addi %b, %x : i32
  func.return %c : i32
}
```

With Zirium installed, run:

```sh
zirium 'filter(op("arith.addi")) | count' operations.mlir
```

The command prints `2`. The shared filter and count behave as they do for
words. Zirium supplies the initial operation stream and the meaning of `op`.
Its parser works on textual MLIR without linking LLVM; name and structure
inspection do not establish full dialect verification.

Following `users` is a domain operation. Zirium resolves the SSA use edges and
keeps the resulting operation handles attached to their document. Equal printed
SSA names in different scopes do not establish identity. Unsupported syntax
can leave facts incomplete; strict parsing and dialect support are host choices.
Zirql supplies neither those edges nor the legality of a source edit.

See the [Zirium query guide](https://github.com/zayenz/zirium/blob/main/docs/query-language.md)
for navigation, recovery, registries and rendering.

## Count MLIR operations by name

The same document contains two additions and one multiplication. Project the
arithmetic operations to their names before tallying:

```sh
zirium 'filter(dialect("arith")) | names | tally | json' operations.mlir
```

```json
{"arith.addi": 2, "arith.muli": 1}
```

Zirium supplies `dialect`, the `names` projection and the JSON emitter. Zirql
supplies the string tally. The common `parse_pipeline` reader does not accept
these host stages; Zirium lowers its own syntax to the typed representation.

Identity and projected equality answer different questions. If navigation
returns the same operation twice, native `unique` can remove that repeated
handle. Two distinct additions survive native `unique`, even though their
names are equal. Projecting `names` first makes both values the string
`arith.addi`, so a string tally counts them together. A word host that enables
text-based unique keys would likewise treat repeated `apple` spellings as
one key. The host must choose and implement that identity rule.

## Build records from labels

Once native items have been projected to strings, both hosts can supply the
same constructed binding `labels = ["alpha", "beta"]` to the value evaluator.
Their accepted surface syntax differs.

For Zincite, save `alpha = 1; beta = 2;` as `labels.dzn`, and this program as
`labels.zq`:

```text
source labels = query("names");
report [(name: n, label: "data:" ++ n) | n in labels];
```

```sh
zincite query --program-file labels.zq labels.dzn
```

For Zirium, save this as `labels.mlir`:

```text
"alpha"() : () -> ()
"beta"() : () -> ()
```

```sh
zirium 'labels = names; [{"name": n, "label": "data:" ++ n} for n in labels] | json' labels.mlir
```

Both lower the generator, ordered record fields and string concatenation to
the shared value evaluator. Before host rendering, the constructed data is:

```json
[
  {"name": "alpha", "label": "data:alpha"},
  {"name": "beta", "label": "data:beta"}
]
```

Zirium prints ordinary JSON. Zincite prints a tagged array and record
representation inside an envelope with `complete`, `limitations` and `result`.
The data agrees before rendering; the printed bytes and inspection metadata
serve different host APIs.

For your own integration, choose native items, identity and domain hooks first.
Then decide which text forms to accept and how to expose results. The
[Rust embedding guide](host-integration.md) describes those boundaries.
