# Presence and lookup

Absence is ordinary data, written `null` or `<>`. It differs from an error and
from unknown native predicate truth. False, zero and empty collections are
present. See [optional data](values.md#absence-lookup-and-defaults).

## absent

`absent(value) -> bool`

Return `true` only for absence: `absent(null)` is `true`, while `absent([])` is
`false`. Errors evaluating the argument still fail. The pattern `absent` has
the same presence test in [value matching](library-control.md#value-match).

## occurs

`occurs(value) -> bool`

Return `true` for a present value: `occurs(false)` is `true`, and
`occurs(lookup({}, "missing"))` is `false`. This is the opposite of `absent`.
It does not test whether a collection is nonempty.

## lookup

`lookup(container, key) -> value or absence`

Read a dictionary, positional array or keyed array. A valid missing entry
returns absence. `lookup({"word": "ash"}, "label")` returns `null`.
Positional keys are one-based integers; keyed keys must match the declared rank
and integer or nominal domains. Dictionaries accept constructed keys, including
collections. Declared schemas and key contracts still apply: an incompatible
container or key fails. A stored `null` and a missing key give the same result;
use `has_key` to distinguish them. Ordinary [access](library-operators.md#access)
instead fails for a missing entry.

## has_key

`has_key(container, key) -> bool`

Test whether an entry is stored, using the same container and key rules as
`lookup`. `has_key({"word": null}, "word")` returns `true`, while
`has_key({}, "word")` returns `false`. A declared keyed axis can contain a key
without a present cell; axis membership alone does not make `has_key` true.
Invalid key types fail rather than return `false`.

## default

`value default fallback -> value`

Evaluate `value` once. Return it when present; otherwise evaluate `fallback`.
`lookup({}, "label") default "unlabelled"` returns `"unlabelled"`.
Empty collections and false do not select the fallback. This operator does not
catch errors: `(1 div 0) default 0` fails. Both operands receive preflight
checks. Parenthesize the intended boundary when composing arithmetic; see
[precedence](library-operators.md#precedence).

## require

`require(condition, value, "message") -> value`

Require a known Boolean condition. True evaluates and returns `value`; false
fails with the literal message without evaluating `value`.
`require(true, 7, "expected a value")` returns `7`.
Unknown truth and non-Booleans fail. Scope, known-type and purity checks still
cover the unselected value. See [requirements](values.md#array-summaries-and-requirements).

## present patterns

`present(pattern)` is a pattern, not a value constructor or ordinary call.

It matches an existing present value and applies the nested pattern to that
value. `match (7) { present(n) => n, _ => 0 }` returns `7`; absence tries the
next arm. A strict binding mismatch fails, while a `case` generator skips it.
See [optional patterns](functions-and-control-flow.md#match-optional-values).
