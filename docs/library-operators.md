# Operators and access

These are the constructed expression operators. Native set operators have a
separate [host-dependent contract](library-native.md#native-set-operators).
All operations share the request's finite work, result and nesting limits.

## Precedence

From strongest to weakest, the common expression reader uses:

| Level | Forms |
| --- | --- |
| Access and update | `a[i]`, `a[i, j]`, `tuple.1`, `row.field`, `with` |
| Optional fallback | `default` |
| Prefix | `not`, unary `+`, unary `-` |
| Multiplicative | `*`, `/`, `div`, `mod` |
| Additive | `+`, `-` |
| Integer range | `..` |
| String concatenation | `++` |
| Ordering and membership | `<`, `<=`, `>`, `>=`, `in` |
| Equality | `==`, `=`, `!=` |
| Conjunction | `and`, `/\` |
| Disjunction | `or`, `\/` |

Binary operators at the same level associate left to right. Pipelines apply to
the completed expression; group a pipeline result before using it as an operand.
Native `union`, `intersect` and `except` have equal precedence and associate
left to right between native pipeline operands. Parentheses make their grouping
explicit. See the [expression guide](values.md#operators-and-precedence).

## Signs, addition and subtraction

`+number`, `-number`, `left + right`, `left - right -> number`

Accept signed 64-bit integers and finite floats. `-3 + 5` returns `2`.
Integer results are checked for overflow, including negating the minimum
integer. A mixed integer/float calculation requires exact binary64
representation of the integer. There is no string-to-number coercion.

## Multiplication and division

`left * right`, `left / right`, `left div right`, `left mod right`

Multiplication accepts integers or finite floats. `/` returns a float, including
for integer operands: `5 / 2` returns `2.5`. `div` and `mod` require integers:
`-5 div 2` returns `-2`, and `-5 mod 2` returns `-1`. Division truncates toward
zero; the remainder has the dividend's sign. Zero divisors, integer overflow,
nonfinite float results and nonzero float results underflowing to zero fail.
Mixed numeric multiplication/division requires exactly representable integers.
See [checked arithmetic](values.md#checked-arithmetic-and-conditions).

## Integer ranges

`lower..upper -> set<int>`

Construct an inclusive ascending integer set. `2..4` contains `2, 3, 4` in that
order; `4..2` is empty. Endpoints must be integers. The full range is eager and
spends the request budget; it does not form an unbounded iterator.

## Equality

`left == right`, `left = right`, `left != right -> bool`

`=` aliases `==` in expressions. Equality requires compatible underlying types;
it does not coerce numbers or text. Arrays and tuples compare by position, sets
by membership and dictionaries by key/value associations. Thus
`{1, 2} == {2, 1}` is `true`. Keyed arrays also retain their typed axes and
present cells. Two absences compare equal; absence differs from a present value.
Declared optional types must remain compatible even when both values are absent.
Native equality remains the host's identity contract. Structural collection-key
identity keeps distinct value kinds separate. See [collection equality](values.md#dictionaries-and-collection-order).

## Ordered comparisons

`left < right`, `left <= right`, `left > right`, `left >= right -> bool`

Compare present integers and finite floats. `2 < 3` returns `true`. Mixed
integer/float comparisons require an exactly representable integer. Strings,
nominal members, Booleans, absence and collection operands fail. String and
nominal ordering are available through sort/extrema instead. These numeric
expressions do not use the structural order used by set and dictionary-key
sorting.

## Membership

`element in set -> bool`

Test structural membership in a constructed set: `{1, 2} in {{2, 1}}` returns
`true`. An empty compatible set returns `false`. A strict scalar or nominal set
requires its declared element type; a general set accepts constructed values.
Arrays and dictionaries are not membership operands; use `has_key` for stored
entries. With host support, `item in seen` also tests a native
[identity set](library-worklists.md#identity_set), preserving owner identity.
The `in` token in a comprehension instead introduces its generator source.

## Boolean operators

`not condition`, `left and right`, `left /\ right`, `left or right`, `left \/ right`

Constructed operands must be known Booleans. `and` skips the right operand after
false; `or` skips it after true. `not false` is `true`. Preflight still checks
both operands. `/\` aliases `and`, and `\/` aliases `or`.
Native predicates can also return unknown: negation preserves it, and a decisive
false conjunction or true disjunction can settle it. Native filter keeps known
true; a native match guard or path target with unresolved truth fails.
See [predicate truth](shared-concepts.md#the-small-embedding-reader).

## Access

`array[index]`, `array[i, j]`, `dictionary[key]`, `row.field`, `tuple.1`

Read one value. Positional arrays and tuples use one-based positions;
`[4, 9][2]` returns `9`, and `(4, 9).1` returns `4`.
`a[i, j]` abbreviates `a[(i, j)]` for a keyed array. Dictionary keys can be
constructed values; dot access names a static string field. Keyed access
retains declared key types, rank and missing cells. A missing position, field or
cell fails. `lookup` supplies optional access. A native field keeps the host's
value and source ownership; it does not implicitly render it.
See [literals and access](values.md#literals-and-access).

## Record update

`record with {field: replacement, ...} -> record`

Return a record with existing fields replaced. `{"limit": 8} with {limit: 12}`
returns `{"limit": 12}`. Evaluate the base once, then replacements in written
order in surrounding scope. Preserve field order, retained contracts, native
children and aliases. Missing or duplicate targets fail. This form does not
insert or remove fields and does not update a general dictionary or keyed array.
See [record updates](values.md#reuse-a-record-with-changed-fields).

## Pipes

`source | stage | stage`

Apply stages left to right to a completed expression. `[3, 1, 3] | unique | count`
returns `2`. Explicit sources supply their own input; bare constructed
filter/map/flat_map/count use the host's immutable constructed input.
Native stages use the host's current native input and type rules. A scalar count
cannot feed a collection stage, and constructed tally is terminal. The generator
pipe in `[x | x in source]` is a separate comprehension delimiter.
See [constructed pipelines](library-pipelines.md) and [native forms](library-native.md).
