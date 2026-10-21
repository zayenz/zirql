# Library reference

Find a shared function, pipeline stage, operator or special form here. The
entries describe supported behavior in this checkout, including the current
extensions. Hosts may pin earlier Zirql versions or expose a smaller frontend;
check their own guides before using a form in a consumer.

A shared operation can support ordinary values, native inputs, or both. Each
entry describes its supported forms and host requirements. Native inputs need
host adapters and retain their objects and source context. Domain-specific
names belong to the [Zirium](https://zayenz.se/zirium/guides/query-language/)
and [Zincite](https://zayenz.se/zincite/guides/queries/) references. Proposed
language/CP forms are outside this library reference.

Each shared operation has one entry covering its constructed and native forms,
including the native projections and result shapes that differ. Each entry gives
its inputs, result, example and relevant empty/error/order rules. All operations
spend the active finite request budget. Preflight scope,
known-type and purity checks apply even to empty input and unselected branches;
runtime errors in skipped branches remain unexecuted. See
[diagnostics and limits](diagnostics-and-limits.md).

## Browse by purpose

- [Scalars and strings](library-strings.md)
- [Presence and lookup](library-presence.md)
- [Operators and access](library-operators.md)
- [Collections and reductions](library-collections.md)
- [Ordering and extrema](library-ordering.md)
- [Grouping and neighbourhoods](library-grouping.md)
- [Collection pipeline stages](library-pipelines.md)
- [Functions and control flow](library-control.md)
- [Queues and native identity sets](library-worklists.md)
- [Shared native forms](library-native.md)
- [JSON and Markdown rendering (Rust API)](library-rendering.md)

Rust adapters can build [direct native composition steps](embedding-programs.md#build-native-composition-steps)
and share [declaration metadata and scalar argument admission](host-integration.md#declaration-metadata-and-scalar-arguments).

The [language reference](language-reference.md) covers grammar and scope;
[the word tutorial](examples.md) and [CSV examples](csv.md) develop full queries.
The [published Rust API](https://docs.rs/zirql/0.2.0/zirql/) describes registry
0.2.0. Run `cargo doc --open` in this checkout for the development APIs used by
these guides. Reader forms and Rust enum variants are not interchangeable:
for example, there is no ordinary `array(...)` call, and nominal member
construction needs a host resolver.

## Alphabetical index

Each spelling links to its detailed entry. Symbols are listed first, followed
by names; aliases share entries. `array`, `dict`, `keyed`, `record` and `tuple`
name literal/type forms, not ordinary calls. Entries marked native require host
support. `present` is a pattern.

| Name or form | Reference |
| --- | --- |
| `!=` | [Operators and access](library-operators.md#equality) |
| `*` | [Operators and access](library-operators.md#multiplication-and-division) |
| `+` | [Operators and access](library-operators.md#signs-addition-and-subtraction) |
| `++` | [Strings](library-strings.md#string-concatenation) |
| `-` | [Operators and access](library-operators.md#signs-addition-and-subtraction) |
| `.` | [Operators and access](library-operators.md#access) |
| `..` | [Operators and access](library-operators.md#integer-ranges) |
| `/` | [Operators and access](library-operators.md#multiplication-and-division) |
| `/\` | [Operators and access](library-operators.md#boolean-operators) |
| `<` | [Operators and access](library-operators.md#ordered-comparisons) |
| `<=` | [Operators and access](library-operators.md#ordered-comparisons) |
| `=` | [Operators and access](library-operators.md#equality) |
| `==` | [Operators and access](library-operators.md#equality) |
| `>` | [Operators and access](library-operators.md#ordered-comparisons) |
| `>=` | [Operators and access](library-operators.md#ordered-comparisons) |
| `[]` | [Operators and access](library-operators.md#access) |
| `\/` | [Operators and access](library-operators.md#boolean-operators) |
| `\|` | [Operators and access](library-operators.md#pipes) |
| `absent` | [Presence and lookup](library-presence.md#absent) |
| `align` | [Grouping and neighbourhoods](library-grouping.md#align) |
| `and` | [Operators and access](library-operators.md#boolean-operators) |
| `array` | [Collections and reductions](library-collections.md#arrays-and-tuples) |
| `axes` | [Collections and reductions](library-collections.md#keyed-arrays) |
| `bind` | [Shared native forms](library-native.md#native-capture-match) |
| `break` | [Functions and control flow](library-control.md#break) |
| `case` | [Collections and reductions](library-collections.md#comprehensions) |
| `check` | [Collection pipeline stages](library-pipelines.md#check) |
| `contains` | [Strings](library-strings.md#contains) |
| `continue` | [Functions and control flow](library-control.md#continue) |
| `count` | [Collection pipeline stages](library-pipelines.md#count) |
| `default` | [Presence and lookup](library-presence.md#default) |
| `dict` | [Collections and reductions](library-collections.md#dictionaries-and-records) |
| `div` | [Operators and access](library-operators.md#multiplication-and-division) |
| `else` | [Functions and control flow](library-control.md#if) |
| `elseif` | [Functions and control flow](library-control.md#if) |
| `endif` | [Functions and control flow](library-control.md#if) |
| `ends_with` | [Strings](library-strings.md#ends_with) |
| `entries` | [Collections and reductions](library-collections.md#entries) |
| `except` | [Shared native forms](library-native.md#native-set-operators) |
| `exists` | [Collections and reductions](library-collections.md#exists) |
| `filter` | [Collection pipeline stages](library-pipelines.md#filter) |
| `fixpoint` | [Shared native forms](library-native.md#fixpoint) |
| `flat_map` | [Collection pipeline stages](library-pipelines.md#flat_map) |
| `fn` | [Functions and control flow](library-control.md#fn-and-calls) |
| `fold` | [Functions and control flow](library-control.md#fold) |
| `for` | [Collections and reductions](library-collections.md#comprehensions) |
| `forall` | [Collections and reductions](library-collections.md#forall) |
| `from` | [Shared native forms](library-native.md#native-capture-match) |
| `function call` | [Functions and control flow](library-control.md#fn-and-calls) |
| `group_by` | [Grouping and neighbourhoods](library-grouping.md#group_by) |
| `has_key` | [Presence and lookup](library-presence.md#has_key) |
| `head` | [Collection pipeline stages](library-pipelines.md#head-and-tail) |
| `identity_set` | [Queues and native identity sets](library-worklists.md#identity_set) |
| `if` | [Functions and control flow](library-control.md#if) |
| `in` | [Operators and access](library-operators.md#membership) |
| `index_set` | [Collections and reductions](library-collections.md#index_set) |
| `input` | [Shared native forms](library-native.md#input) |
| `insert` | [Queues and native identity sets](library-worklists.md#insert) |
| `intersect` | [Shared native forms](library-native.md#native-set-operators) |
| `keyed` | [Collections and reductions](library-collections.md#keyed-arrays) |
| `length` | [Collections and reductions](library-collections.md#length) |
| `let` | [Functions and control flow](library-control.md#let) |
| `lookup` | [Presence and lookup](library-presence.md#lookup) |
| `loop` | [Functions and control flow](library-control.md#loop) |
| `map` | [Collection pipeline stages](library-pipelines.md#map) |
| `map_by` | [Shared native forms](library-native.md#map_by) |
| `match (value)` | [Functions and control flow](library-control.md#value-match) |
| `match {captures}` | [Shared native forms](library-native.md#native-capture-match) |
| `max` | [Ordering and extrema](library-ordering.md#min-and-max) |
| `max_all` | [Ordering and extrema](library-ordering.md#all-ties-extrema) |
| `max_all_by` | [Ordering and extrema](library-ordering.md#all-ties-extrema) |
| `max_by` | [Ordering and extrema](library-ordering.md#min_by-and-max_by) |
| `min` | [Ordering and extrema](library-ordering.md#min-and-max) |
| `min_all` | [Ordering and extrema](library-ordering.md#all-ties-extrema) |
| `min_all_by` | [Ordering and extrema](library-ordering.md#all-ties-extrema) |
| `min_by` | [Ordering and extrema](library-ordering.md#min_by-and-max_by) |
| `mod` | [Operators and access](library-operators.md#multiplication-and-division) |
| `not` | [Operators and access](library-operators.md#boolean-operators) |
| `occurs` | [Presence and lookup](library-presence.md#occurs) |
| `one` | [Collections and reductions](library-collections.md#one) |
| `or` | [Operators and access](library-operators.md#boolean-operators) |
| `param` | [Functions and control flow](library-control.md#param) |
| `parse_float` | [Scalars and strings](library-strings.md#parse_float) |
| `parse_int` | [Scalars and strings](library-strings.md#parse_int) |
| `paths` | [Shared native forms](library-native.md#paths) |
| `pop_front` | [Queues and native identity sets](library-worklists.md#pop_front) |
| `present` | [Presence and lookup](library-presence.md#present-patterns) |
| `project` | [Shared native forms](library-native.md#project) |
| `push_back_all` | [Queues and native identity sets](library-worklists.md#push_back_all) |
| `queue` | [Queues and native identity sets](library-worklists.md#queue) |
| `record` | [Collections and reductions](library-collections.md#dictionaries-and-records) |
| `render_json` (Rust) | [JSON rendering](library-rendering.md#render_json) |
| `render_json_value` (Rust) | [JSON rendering](library-rendering.md#render_json_value) |
| `render_markdown` (Rust) | [Markdown rendering](library-rendering.md#render_markdown) |
| `render_report_markdown` (Rust) | [Mixed Markdown reports](library-rendering.md#render_report_markdown) |
| `report` | [Functions and control flow](library-control.md#report) |
| `require` | [Presence and lookup](library-presence.md#require) |
| `reverse` | [Collection pipeline stages](library-pipelines.md#reverse) |
| `runs_by` | [Grouping and neighbourhoods](library-grouping.md#runs_by) |
| `scan` | [Functions and control flow](library-control.md#scan) |
| `set` | [Collections and reductions](library-collections.md#sets) |
| `sort` | [Ordering and extrema](library-ordering.md#sort) |
| `sort_by` | [Ordering and extrema](library-ordering.md#sort_by) |
| `starts_with` | [Strings](library-strings.md#starts_with) |
| `sum` | [Collections and reductions](library-collections.md#sum) |
| `tail` | [Collection pipeline stages](library-pipelines.md#head-and-tail) |
| `tally` | [Collection pipeline stages](library-pipelines.md#tally) |
| `to_json_value` (Rust) | [JSON rendering](library-rendering.md#to_json_value) |
| `to_report_json_value` (Rust) | [Mixed native reports](library-rendering.md#mixed-native-reports) |
| `to_string` | [Scalars and strings](library-strings.md#to_string) |
| `tuple` | [Collections and reductions](library-collections.md#arrays-and-tuples) |
| `union` | [Shared native forms](library-native.md#native-set-operators) |
| `unique` | [Collection pipeline stages](library-pipelines.md#unique) |
| `where` | [Collections and reductions](library-collections.md#comprehensions) |
| `where_exists` | [Shared native forms](library-native.md#where_exists) |
| `windows` | [Grouping and neighbourhoods](library-grouping.md#windows) |
| `with` | [Operators and access](library-operators.md#record-update) |
| `zip_exact` | [Collections and reductions](library-collections.md#zip_exact) |
