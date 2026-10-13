# Query examples

These examples show the data a query starts with and the value it produces.
The word pipeline uses Zirql's common text reader. The MLIR query and report
examples use the Zincite and Zirium frontends, which lower their different
syntax to shared evaluation.

## Count MLIR operations by name

Save this complete input as `operations.mlir`:

```text
func.func @mix(%x: i32, %y: i32) -> i32 {
  %a = arith.addi %x, %y : i32
  %b = "arith.muli"(%a, %y) : (i32, i32) -> i32
  %c = arith.addi %b, %x : i32
  func.return %c : i32
}
```

Select the arithmetic operations and count their names:

```sh
zirium 'filter(dialect("arith")) | names | tally | json' operations.mlir
```

`names` is Zirium's projection from operation handles to strings. `tally` counts
those strings and returns keys in sorted order. `json` is Zirium's emitter:

```json
{"arith.addi": 2, "arith.muli": 1}
```

The count uses printed operation names because those are the explicitly
projected keys. It does not claim that the two additions are the same operation.
A native `unique` query would instead use the host's operation identity.

This syntax belongs to Zirium's frontend. Zirql's common text reader does not
parse `names`, `tally` or `json`; the host lowers its query to typed stages.
See the [Zirium query guide](https://github.com/zayenz/zirium/blob/main/docs/query-language.md)
for the domain stages.

## Select and count borrowed words

The input is an ordered sequence of words borrowed from `birch apple ash`.
The host defines `prefix("a")` as a test of whether a word starts with `a`.

```text
filter(prefix("a")) | head(1) | count
```

| Step | Result |
| --- | --- |
| Input | `birch`, `apple`, `ash` |
| `filter(prefix("a"))` | `apple`, `ash` |
| `head(1)` | `apple` |
| `count` | `Count(1)` |

The input order determines which word `head(1)` keeps. Filtering selects only
known true predicate answers; an Unknown answer would not keep a word.
`count` is terminal, so another stage after it is rejected before evaluation.

The [complete Rust host example](../README.md#a-borrowed-source-query) implements
this predicate and parser callback, admits the input with a budget and evaluates
the pipeline. It returns a native count before any rendering. See
[pipelines](shared-concepts.md) for the language rules and
[embedding in Rust](host-integration.md) for the adapter requirements.

## Build records from labels

For Zirium, save this complete input as `labels.mlir`:

```text
"alpha"() : () -> ()
"beta"() : () -> ()
```

For Zincite, save `alpha = 1; beta = 2;` as `labels.dzn`. Its `names` query
projects the assignment names in source order. Save this program as
`labels.zq`:

```text
source labels = query("names");
report [(name: n, label: "data:" ++ n) | n in labels];
```

Run `zincite query --program-file labels.zq labels.dzn`.
Zirium accepts the program inline:

```text
labels = names;
[{"name": n, "label": "data:" ++ n} for n in labels] | json
```

```sh
zirium 'labels = names; [{"name": n, "label": "data:" ++ n} for n in labels] | json' labels.mlir
```

Both construct this ordered value, shown as JSON for readability:

```json
[
  {"name": "alpha", "label": "data:alpha"},
  {"name": "beta", "label": "data:beta"}
]
```

Each generator binds `n` to one projected string. The head creates a record
with `name` first and `label` second; `++` concatenates strings. Array order
follows the source. A native operation handle must be projected to a string
before this expression can concatenate it.

The constructed records agree before rendering. Zincite's report renderer
retains its tagged envelope and completeness information; Zirium's JSON emitter
renders ordinary JSON. The core evaluates the lowered value expression, while
the hosts parse these surfaces and choose their output formats.

## Filter before constructing a value

For the integer array `[2, 3, 4]`, this expression-tree notation filters values
larger than 2, then doubles each surviving value:

```text
[x * 2 for x in [2, 3, 4] if x > 2]
```

The result is the positional array `[6, 8]`. Its positions are 1 and 2; the
filter does not preserve the input positions 2 and 3 as keys. Use explicitly
[keyed arrays](values.md#keyed-arrays) when index identity must survive.

This is notation for the supported Rust comprehension representation, not
syntax accepted by `parse_pipeline`. A frontend supplies the text syntax.
The evaluator charges all three generator candidates, including the rejected
value 2, before evaluating the two heads. See
[values and comprehensions](values.md) for scope and arithmetic rules.

## Examples of unimplemented language forms

The [design examples](language-direction.md) explore per-function operation
reports, capacity checks, native captures, grouping and bounded explanatory
paths. Their projection, reducer, parameter, requirement, match and path forms
are not implemented. They describe the intended language direction, rather
than commands to run against the current library.
