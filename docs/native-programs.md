# Programs and native data

A single expression is enough for each analysis in the [word
tutorial](examples.md). A program lets several analyses share named intermediate
results. Its bindings can hold constructed values, native selections or records
of related native objects, and its statements execute in written order.

The host supplies statement and output handling. Zirql's common readers
recognize statement prefixes and expressions; a host can also translate its own
syntax into the same program representation. The [word
executable](getting-started.md) accepts one expression; it does not parse the
complete programs or native extensions shown here.

## Statements and scope

For a frontend using the common statement prefixes, a small report can read:

```zirql
let selected = [w for w in words if starts_with(w, "a")];
report {"words": selected, "count": length(selected)};
```

With the word model, the report contains `ash`, `apple`, `apple` and a count of
3. `let` installs an immutable name. Later statements can use it, but earlier
statements cannot refer forward to it. Duplicate definitions and shadowing of
existing bindings fail before execution.

Each statement starts with the original top-level native input. A previous
statement does not silently replace that input. Bind its result and refer to
the binding explicitly when a later statement should use it.

The host defines what `report` does with the result, including formatting and
whether it produces output at all for a library call. Single expressions need
no file or named entry point in the shared language representation.

## Required caller parameters

Required parameters let a caller supply typed data without constructing query
text:

```zirql
param prefix: string;
let selected = [w for w in words if starts_with(w, prefix)];
report {"words": selected, "count": length(selected)};
```

Parameter types are `int`, `string` and `bool`. Missing, duplicate, extra or
incompatible supplied values fail before domain execution. Defaults are not
supported. A parameter becomes available at its written declaration and follows
the same no-shadowing rule as other bindings.

Library callers can pass typed values. A host CLI using the shared `name=value`
decoder accepts these forms:

| Type | Decoding |
| --- | --- |
| `string` | Preserve all text after the first `=`, including spaces, quotes and additional `=` characters |
| `int` | Optional sign followed by decimal digits fitting a signed 64-bit integer |
| `bool` | Exactly `true` or `false` |

Values are not substituted into the program's source. The host documents how
to pass them through its command-line interface.

## Saved native values

Saving a native selection retains its objects and their source context. A word
occurrence can remain attached to the source text, and a document handle can
remain attached to its owning document. Saving or reading the binding does not
render it.

A record can retain several related native values. Field selection preserves
the selected field's native kind. A later projection decides which properties
become scalar report data.

## Construct ordered report rows

The remaining examples assume a richer word model with native occurrences.
Suppose its host supplies `text` and `byte_length`, each producing one scalar
per occurrence. A projection can build a row for each word:

```zirql
project(w, {
  "word": one(w | text),
  "bytes": one(w | byte_length)
})
```

On `ash apple birch apple`, this gives rows `ash/3`, `apple/5`, `birch/5`,
`apple/5`. Both occurrences of `apple` survive. `w` is local to the projection;
its fields evaluate in written order against the same native subject.

`one` requires exactly one projected value. Empty or multiple results fail. A
missing domain operation or incompatible field type fails preflight even when
the input selection is empty. These operations are an [illustrative extension of
the word model](language-direction.md); they are not implemented by the tutorial
executable.

## Optional projected fields

Value expressions inside a projection use the same optional-data rules as other
expressions. If the host supplies `metadata` as a single record for one word:

```zirql
project(w, {
  "word": one(w | text),
  "label": lookup(one(w | metadata), "label") default "unlabelled"
})
```

A missing `label` gets the fallback. An error obtaining `metadata` still fails;
`default` handles absence, not operation failures. All branches retain scope,
kind and purity checks before any native field executes.

## Retain native captures

Native matching enumerates related objects and keeps them together in a record.
An occurrence host could supply `has_next`, `next_word` and `same_text`:

```zirql
match {
  from word in (input | filter(has_next));
  bind following = next_word(word);
  where same_text(word, following);
}
```

For `ash apple apple birch`, this finds the adjacent `apple` occurrences. The
capture has separate `word` and `following` fields with distinct source positions.
The adapter defines those operations; the core defines the clause behavior:

| Clause | Meaning |
| --- | --- |
| `from name in query` | Enumerate candidates in native order |
| `bind name = query` | Require exactly one native subject for each candidate |
| `where predicate` | Continue only when the final predicate answer is known true |

Later clauses see earlier names. A name is absent in its own source, and the
rightmost `from` varies fastest. Captures stay local to the match. Duplicate
names, shadowing and a binding named `input` fail.

A false guard rejects a candidate. An unknown final guard or a failed required
binding is an error. Empty input does not bypass preflight. Captured handles
retain their owner until an explicit projection creates scalar data.

Value matching, written `match (value) { ... }`, instead selects one result.
See [functions and control flow](functions-and-control-flow.md).

## Explain native connections

A host can expose a named edge for bounded path queries:

```zirql
paths(next_word, to=prefix("birch"), max_depth=2)
```

With a native source occurrence `ash` in `ash apple birch`, this returns one
witness with steps `[ash, apple, birch]`. The host supplies the `next_word` edge,
the prefix predicate and occurrence identity. `max_depth` is a required
nonnegative integer measured in edges. An edge is one registered identifier,
not a pipeline.

The result has `source`, `sink` and `steps` fields. Steps include both endpoints;
a source that already matches the target has a zero-edge witness `[source]`.
For each source/sink pair, Zirql chooses one shortest witness within the bound.
Sources retain incoming identity order, sinks follow breadth-first discovery,
and native neighbour order breaks ties between equally short witnesses.
Repeated handles and cycles do not duplicate a pair. Searching continues through
matching sinks.

A bound of one finds no witness from `ash` to `birch` here. That does not prove
global disconnection. An unknown final target predicate fails, and separate
host limitations remain attached to the request. Exhaustion is an error rather
than a truncated set of witnesses.

## Written and implicit output

A host can define terminal output, which ends an expression, or pass-through
output, after which stages can continue. It can also choose implicit output for
a final expression without a written emitter. The host's guide specifies that
policy and its syntax.

Pure bindings, constructed operands and functions cannot emit or edit. Explicit
host effects have the host's transaction rules; a later failure does not
necessarily undo output already emitted. See [diagnostics and limits](diagnostics-and-limits.md#effects-and-output).

<span id="copying-saved-values"></span>
<span id="repeated-projection"></span>
<span id="named-pure-functions"></span>
<span id="execution-costs"></span>

Named declarations, recursion and patterns have their own
[language chapter](functions-and-control-flow.md). Rust copying, preparation
and output contracts are covered by [host integration](host-integration.md),
[program construction](embedding-programs.md) and [execution costs](computation.md).

## Order native selections

Hosts lower native `sort`, `min`, `max`, `min_all` and `max_all` stages through
`ValueReaderHost::ordering`. Their `*_by(query)` forms retain a
singleton-relative selector with the containing expression's lexical scope.
Typed adapters construct `NativeOrdering<Q>` directly. The shared algorithm
orders native payloads without rendering them or replacing their owner.

Sorting is stable. Single extrema keep the first equal-key item; all-ties
extrema keep every equal-key item in input order. Selectors run once per item in
input order before comparisons begin, including for extrema. The existing
`NativeOrderKey::Value` shares constructed scalar admission and comparison:
present homogeneous signed integers, finite floats, exact strings or members
of one nominal type. Every key is admitted before comparison; inconsistent
nominal names, member metadata or declaration order fail even in a losing key.
Native `Count` and `Text` descriptors keep their separate domain: counts retain
`usize` range and sort before text. They cannot mix with typed `Value` keys.
Hosts decide their admitted kinds and empty policy; the shared algorithm itself
returns an empty selection on empty input.

## Correlated native dictionaries

`map_by(key_query, value_query)` evaluates two independent queries against each
native input item. The key must produce exactly one string. Inputs run in their
original order, key before value; a duplicate key fails before its value runs.
Values retain their native kind and owner, including nested dictionaries and
counts. The host checks supported kinds and pure selector policy before
execution, even for empty input.

The callable constructor returns `Vec<(String, V)>` in insertion order.
`sort_map_by_value` reorders those entries stably; equal values keep their
current order. `select_map` applies reverse/head/tail to the same dictionary
kind, including an unsorted dictionary. Values remain native throughout.

Its scalar projection accepts homogeneous null, Boolean, numeric or string
values. Signed/unsigned integers and finite floats share the numeric category
and compare exactly. Constructed dictionary selectors use the narrower
constructed ordering contract: homogeneous signed integers, finite floats,
strings or one nominal member type, with no integer/float mixing.

## Accepted collection and native alignment

The shared library keeps insertion order for scalar sets and string dictionaries.
Set duplicates retain their first representative. Constructed sorting returns
the same collection kind. Dictionary equality compares associations and set
equality compares membership, independently of stored order. Explicit keyed
axes retain their shape contract; native fixed-point streams still use ordered
equality.

For example, these expressions run with the word example host:

```zirql
sort({"z": 2, "a": 2, "m": 1})
sort_by({"z": 2, "a": 2, "m": 1}, value)
sort_by({"z": 2, "a": 2, "m": 1}, value, descending=true)
```

They return dictionaries ordered `a, m, z`, `m, z, a` and `z, a, m`.
Equal keys retain their incoming order in both directions. Sorting again after
reversal keeps the reversed order within ties. Tally fields follow the first
occurrence of each spelling. Empty dictionaries and zero bounds retain the
dictionary kind. A dictionary never implicitly becomes a list of entry records.

Host adapters retain this entry order through saved results, copies and nested
values. JSON object member order is observable here: a renderer must iterate
the retained entries rather than collect them into an automatically sorted map.
Native handles and source coordinates survive until explicit rendering.

Zirium retains ordered dictionaries through its library and JSON results and
uses operand-order native set algebra. Zincite uses shared typed native ordering,
operand-order sets and first-seen tally dictionaries through its pinned Git
dependency. Its `map_by` dictionaries support stable value sorting and bounds;
native and constructed dictionaries render as ordered JSON objects. Later common
reader spellings remain separate work. Native identity, effects, completeness
and data-edit policies stay with each host.

## Native existence

Supporting native hosts read `where_exists(query)` at the start of an expression,
in a group or after a pipeline. The pure relation runs against each singleton
subject with the current lexical scope. A nonempty native result keeps that
original subject, preserving its order and duplicates. A complete empty result
rejects it. Unresolved nonemptiness and nested errors fail the query, including
a failure after earlier matches. Hosts preflight kinds, selectors and effects
even when the incoming stream is empty. Zirium supports ordinary and projected
diff relations; Zincite supports pure native report relations. Zincite normalizes
a singular node or declaration to its possibly empty selection kind.

## Native fixed points

A supporting native host reads `fixpoint(query)` through the common reader.
Each iteration replaces the current selection with the body's result. Equality
is positional and includes duplicates; intermediate selections do not accumulate.
The final body execution establishes an unchanged result. Repeating a non-fixed
selection fails, using one bounded cycle checkpoint.

Zincite supports this accumulating form:

```zirql
let seed = declarations | filter(name("C"));
seed | fixpoint(input union referenced_declarations)
```

The union explicitly retains the current members. Zincite bodies are pure
report computation and must return the same admitted native kind. Owners,
identity and the active budget survive every iteration. Partial adjacency
cannot prove convergence. Zincite uses complete-or-error adjacency and refuses
saved partial selections at their responsible locations while retaining other
report limitations. Domain closure,
paths and specialized reachability remain separate operations.
