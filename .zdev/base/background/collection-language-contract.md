# Collection language contract

The area brief owns the shared language decisions. This contract retains their
syntax, host boundaries and concrete examples for the collection-reader and
consumer-alignment tasks. Public guides describe executable support; this file
also specifies the selected forms that those tasks must deliver.

## Collection forms

Ordinary sets and dictionaries use bare braces, suited to short command-line
expressions:

```text
{3, 1, 3}                              # set, iterating as 3, 1
{x mod 3 for x in 1..5}                # set, iterating as 1, 2, 0
{"pear": 3, "apple": 7}                # dictionary, pear then apple
{w: 1 for w in words}                  # computed keys; duplicates fail
{{"ash", "apple"}: 1}                  # a set used as a dictionary key
{[1, 2], (1, 2), {1, 2}}               # a set of constructed values
{}                                     # empty dictionary
set<int>()                             # explicitly typed empty set
```

A colon at the brace body's own level selects a dictionary; a nonempty body
without a colon selects a set. Each key is an ordinary expression, so `{k: v}`
reads the binding `k`; static string keys stay quoted. Nested collections keep
their own delimiters. Empty braces select a dictionary. The exceptional
`set<int>()`, `set<bool>()`, `set<float>()` and `set<string>()` forms supply an
empty set with an explicit scalar element type; `set<value>()` supplies an empty
general set. A host can supply a typed empty nominal set directly; no common model or member-name resolver is introduced.

String dictionaries reuse the existing ordered string-field objects and their
structural schemas. There is no separate everyday record syntax. `.field`
continues to select a static string field; bracket access and `lookup` accept
any constructed key. General dictionaries use ordered constructed key/value
pairs; all-string dictionaries retain the existing record representation and
empty `{}` remains a record. Keys and values include scalars, absence, tuples,
lists, sets and dictionaries. `dict<K, V>` declares their respective contracts;
`value` admits any constructed value. Integer and nominal indexed arrays remain
a distinct array-shaped form retaining typed axes. No key is coerced to text.

Sets preserve the first occurrence of each identity. Dictionaries preserve
entry insertion order and refuse duplicate keys before evaluating that entry's
value. Construction, iteration, saved values and rendering agree on this order.
Inferred sets accept arbitrary constructed elements, including mixed kinds and
nested collections. Homogeneous scalar sets retain their precise element type;
general sets use `SetElementType::Value`. Explicit scalar contracts remain strict.
Key equality is structural: lists and tuples compare by position, sets by
membership, and dictionaries by associations. Different value kinds remain
distinct, including integers and floats. Iteration order remains observable
separately from set and dictionary equality. Tally uses this key equality and
returns a dictionary, with empty results remaining dictionaries. String tally
retains record results; general tally keeps actual constructed keys.

Both kinds support explicit stable sorting and retain their collection kind:

```text
sort({3, 1, 2})
sort({"pear": 3, "apple": 7})
sort_by({"pear": 3, "apple": 3}, value)
```

These produce a set ordered 1, 2, 3; a dictionary ordered apple, pear; and a
dictionary retaining pear, apple because the values tie. `sort(d)` orders
structural keys, including mixed and nested collections. Lists/tuples compare
lexicographically; sets and dictionaries compare canonical membership and
associations. Distinct kinds remain distinct, while floating signed zeros share
identity and hash. `constructed_value_hash` and `compare_constructed_values`
expose the same budgeted value semantics. Construction and tally use hash
collision buckets, and general dictionary storage lazily indexes repeated
lookups. `sort_by(d, value)` orders scalar values through a closed built-in
selector. For record-valued dictionaries, `sort_by(d, "cost")` selects that
field from each value. `descending=true` reverses comparison direction while
keeping the current order within ties. Unsupported or absent sort keys fail.
Sorting a dictionary retains its kind. JSON remains a host boundary: string
dictionaries can become ordered JSON objects, while general dictionaries need
an explicit host encoding or a serialization error. Output-format restrictions
do not restrict the language key domain.

Array-shaped indexed values remain distinct from insertion-ordered dictionaries:

```text
[2: {"cost": 3}, 5: {"cost": 7}]
[2: {"cost": 3}, 5: {"cost": 7}] axes([5, 2, 9])
[(g, j): j for g in guests for j in [8, 1] if j == 8]
    axes(guests, [8, 1])
[] axes(set<int>(), [8, 1])
```

Keys normally establish their integer or nominal dimensions. Optional declared
index sets retain their full order and unused keys for both empty and nonempty
results. The explicit integer example retains axis 5, 2, 9, stores cells 5 then
2, and leaves 9 missing. Multidimensional arrays remain row-major rather than
preserving arbitrary cell insertion order. Each tuple key supplies one component
per axis. String keys belong to ordinary dictionaries, not these array axes.

Evaluate declared axis sources once, left to right, in surrounding scope before
entries or generators. Sources are integer/member arrays or typed sets; a typed
empty source establishes an empty axis. An untyped empty positional array does
not establish its key identity. Supplied nominal values retain their host's
identity and metadata. Duplicate axis keys, wrong rank or keys outside declared
axes fail. Without declared axes, zero emitted cells must not silently become a
positional array when keyed shape is unestablished. `[]` alone remains the empty
positional array.

The existing shared comprehension API's `empty_axis_sources` is only a
zero-cell fallback. It cannot implement these always-retained declarations.
Later implementation needs a located explicit axis-source path on the existing
keyed nodes, while preserving established consumer fallback behavior. It also
needs insertion-ordered set construction, ordered computed string entries and
sorting that retains set/dictionary kind; those are changes to current code,
not claims about functionality already available.

Blocks keep their explicit function/control positions. A final dictionary or
set inside a block uses its own braces, such as
`if test { {"yes": 1} } else { {} }`. Existing generator scope, repeated `for`,
optional `if`, access/operator precedence and grouping remain unchanged. A
pipeline follows the completed collection expression, including its axes.
All parsing, key/axis work, comparisons, copies and rendering share the active
budget and retain expression locations and native owners.

Zincite may keep its natural MiniZinc literal and comprehension spelling while
lowering to the same shared behavior. Its model facts, target shape validation,
serialization and atomic edit rules stay with the host. This design introduces
no implicit model evaluation, native reflection or broader data-edit policy.

## Native operation alignment

Sets and dictionaries preserve insertion order. Repeated set elements keep
their first position; duplicate dictionary keys remain errors. Sorting returns
the same collection kind with a new iteration order. A dictionary stays a
dictionary after `sort_by(value)`. Equal sort keys retain the incoming order,
including after a prior sort or reversal. Saving, copying, nesting and rendering
a collection must preserve its current order.
Collection value equality retains its membership/key-value meaning rather than
accidentally comparing storage order. Explicit keyed axes keep their shape
contract; native fixed-point streams still use ordered equality.

Native `map_by` dictionaries acquire key order from input traversal. Histograms
acquire key order from the first occurrence of each spelling. `reverse`,
`head(n)` and `tail(n)` preserve the order of retained entries; reversal also
reverses ties. Empty dictionaries remain dictionaries, and zero bounds select
an empty dictionary. No separate ranked-map language value or implicit
conversion to entry records is needed.

For declarations named A, B and C with use counts 1, 2 and 0, the proposed
Zincite report is:

```zirql
let ds = declarations;
report ds | map_by(names, uses | count)
          | sort_by(value) | reverse | head(2);
```

Its JSON output is a dictionary in the selected order:

```json
{"complete":true,"limitations":[],"result":{"B":2,"A":1}}
```

JSON object member order is observable output here. The library must retain
that order before serialization; constructing a default sorted JSON map would
lose it. Native payloads remain native until the explicit rendering boundary.
Existing keyed-array `entries(a)` does not by itself provide `entries(dictionary)`.
Literal dictionary spelling and optional declared index sets have their own
accepted design; native `map_by` still requires string keys.

Zincite native ordering and dictionary value sorting use present homogeneous
signed integers, finite floats, exact strings or members of one nominal type.
Integers and floats do not mix. Missing/absent keys, Boolean keys and incompatible
nominal identities fail. Nominal comparison uses declaration order and matching
identity/metadata. Native counts enter this typed contract through checked
signed conversion. Reuse constructed admission/comparison and the existing
sort/extrema algorithms through a small typed-key path. Native handles require
explicit scalar projections, not debug text or printed-name identity.

Native `min`/`max` and their selector forms retain the first tied item;
`min_all`/`max_all` and their selector forms retain all ties in input order.
New Zincite native extrema return a same-kind empty selection on empty input.
Constructed extrema keep their optional absent-on-empty contract. Known
unsupported kinds, selectors and effects still fail preflight on empty input.

Native set union keeps first appearance from left then right. Intersection and
difference deduplicate in left order. Both operands use the same incoming
selection and execute left then right. Handles keep their established host
identity and owner, including diff sides. Initial operation/diff selections can
start in document/canonical order and navigation retains its domain edge order;
that source order is distinct from ordering the result of a set expression.
A host migration must make any changed set-result order explicit.

This algebra applies to admitted identity-bearing native streams. Literal
streams without a defined identity, counts, dictionaries, constructed arrays
and computational `identity_set` values are different kinds. No identity is
invented from rendering. Constructed scalar sets also acquire insertion order
and same-kind explicit sorting; their existing type and nominal identity checks
remain. Internal worklist identity sets keep their computational contract.

`where_exists(query)` is the selected common reader spelling for the existing
correlated existence algorithm. It retains original incoming handles, order
and duplicates. A nonempty pure singleton-relative relation keeps the subject;
a known complete empty relation rejects it. Errors or unresolved nonemptiness
fail with their locations rather than becoming false. The reader hook and
Zincite adapter remain to be implemented.

Direct native syntax is the normal Zincite form. Remove its obsolete
`query("...")` wrapper in the bounded frontend adoption; no compatibility
adapter is required. `let` remains its sole binding keyword. Existing
original-input reads, lexical scope, pure functions/blocks, terminal Zincite
emitters, host completeness and atomic data-edit rules remain unchanged.
