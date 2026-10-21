# Scalars and strings

The matching operations accept constructed strings and use exact, case-sensitive
text, without Unicode normalization, case folding or regular expressions.
An empty search string matches any string. Non-string arguments fail.
See [string matching](values.md#exact-string-matching) for Unicode details.

## contains

`contains(text, fragment) -> bool`

Return whether `fragment` occurs anywhere in `text`.
`contains("apple", "pp")` returns `true`; `contains("", "")` returns `true`.

## starts_with

`starts_with(text, prefix) -> bool`

Return whether `text` begins with `prefix`. `starts_with("Apple", "a")` returns
`false`. Inside a constructed predicate, `starts_with("a")` abbreviates
`starts_with(_, "a")`; `["ash", "birch"] | filter(starts_with("a"))` returns
`["ash"]`. A native host can supply its own predicate leaves instead.

## ends_with

`ends_with(text, suffix) -> bool`

Return whether `text` ends with `suffix`. `ends_with("ash", "sh")` returns
`true`; `ends_with("ash", "")` returns `true`.

## String concatenation

`left ++ right -> string or positional array`

Join two strings in operand order. `"ash" ++ "!"` returns `"ash!"`; an empty
string contributes no text. The same operator joins two positional arrays:
`[1, 2] ++ [3]` returns `[1, 2, 3]`, with fresh one-based positions and retained
element order and duplicates. An empty array contributes no elements.

Both operands must be strings or both positional arrays. No value is implicitly
converted to text; sets, tuples, dictionaries and keyed arrays are refused.
Ordinary native wrappers follow the same admitted data forms. See
[operator precedence](library-operators.md#precedence).

## parse_int

`parse_int(text: string) -> int`

Parse an optional ASCII `+` or `-` followed by one or more ASCII decimal digits.
Leading zeros are accepted: `parse_int("+002")` returns `2`. The result must fit
signed 64-bit, including `-9223372036854775808` and `9223372036854775807`.
Empty or sign-only strings, whitespace, separators, prefixes, decimal points,
exponents and trailing text are errors. No trimming or implicit coercion occurs.

## parse_float

`parse_float(text: string) -> float`

Parse an optional sign, one or more ASCII decimal digits, an optional dot followed
by one or more digits, and an optional `e`/`E` exponent with optional sign and at
least one digit. `parse_float("12.5e-1")` returns `1.25`. `.5`, `1.`, `1e`,
whitespace, trailing text and nonfinite spellings such as `NaN` or `inf` are refused.

The result is correctly rounded finite binary64. Overflow and nonzero decimals
rounded to zero are errors. Finite subnormals are accepted; `parse_float("5e-324")`
returns the smallest positive subnormal. Signed decimal zero retains its sign,
including `parse_float("-0e-400")`.

## to_string

`to_string(value: bool | int | float | string) -> string`

Booleans use `true`/`false`, integers use signed decimal, and finite floats use
locale-independent shortest round-trippable decimal text accepted by `parse_float`.
Negative floating zero remains `-0.0`. Strings retain their content:
`"count=" ++ to_string(2)` returns `"count=2"`.

Absence, collections, nominal members and native values are refused. Project a
native value to a scalar first; use the explicit renderers for whole values.
Declared wrappers are transparent for all three conversions. Preflight checks
known operand types even in unused functions, empty input and skipped branches.
Parsing string contents happens only when evaluated, so
`if true then 1 else parse_int("bad") endif` returns `1`.
Errors retain the conversion's source location and spend the active request budget.

Rust callers construct `ValueExpressionKind::Unary` with
`ValueUnaryOperator::ParseInt`, `ParseFloat` or `ToString` and the operand expression.
These conversions are newer than published Zirql 0.2.0; use this checkout or a
consumer revision that explicitly supports them.
