+++
schema_version = 1
id = "base-051"
key = "shared-native-map-construction"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Share correlated native map construction

## Outcome

Zirql constructs string-keyed maps from correlated native queries, retaining values until the host rendering boundary.

## Context

Zirium has three map_by loops: query.rs::evaluate_host_stage builds legacy JSON maps, query/builder.rs::evaluate_structured builds NativeValue::Map, and query/diff_program.rs::map retains diff values. Shared projection/records and group_by do not supply unique-string-key map construction from independent key/value queries. Read query-language.md Counting values and building maps and query-dsl.md Nested queries and native maps.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [x] A shared callable representation and common map_by(key_query, value_query) reader evaluate both queries against the same singleton input, in input order, with existing saved-binding and prepared-call scopes.
- [x] Keys must produce exactly one string; duplicate keys fail before evaluating that duplicate entry’s value. Empty input returns an empty map and map keys have lexical order. Existing restrictions on editing/emitting inside selectors remain enforced.
- [x] Zirium text, typed and diff paths delegate iteration, key/cardinality validation and duplicate handling to Zirql. Typed and diff map values retain native selections, types, attributes, counts and nested maps; legacy textual output retains its documented JSON shape through an explicit host conversion.
- [x] Nested evaluation, native ownership/side checks, map entry/storage admission, copied values and failures share the active limits and query locations. A second small host can use the shared map construction without implementing that loop.

## Validation

- Reuse map_by controls for empty input, missing/multiple/duplicate keys, nested histograms, retained operation handles and later-error buffering. Compare public text/typed/diff outputs without requiring identical host renderings.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.

## Result

Share correlated native map construction and common reading; preserve native values and host rendering boundaries.

Validation:

- Independent whole review 1 passed with no corrections; exact snapshot W5e9fd8e497444bff unchanged.
- Stable and Rust 1.88 formatting, strict Clippy, workspace tests, doctests and rustdoc passed in both affected repositories.
- CPython 3.11–3.14 affected query/diff suites, Ruff and ty passed; private extensions and real pinned Git source verified.
- Production docs checks, 19-page link check and desktop/mobile UI acceptance passed; git diff and zdev checks passed.
