# Strings

These operations accept constructed strings. Matching uses exact, case-sensitive
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

`left ++ right -> string`

Join two strings in operand order. `"ash" ++ "!"` returns `"ash!"`.
An empty operand contributes no text. Numbers and collections are not converted
to strings. See [operator precedence](library-operators.md#precedence).
