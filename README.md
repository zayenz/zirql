# Zirql

Zirql 0.1.3 is a Rust library for bounded query programs over application data.
It supplies a shared expression reader and evaluator, immutable bindings,
comprehensions, collection operations and typed pure functions. Native objects
retain their host's ownership, identity and source coordinates throughout a
query. Rendering and effects belong to the application.

[Zirium](https://zayenz.se/zirium/) uses Zirql for MLIR queries;
[Zincite](https://zayenz.se/zincite/) uses it for MiniZinc queries. The included
word and CSV examples show smaller hosts using the same library. Zirql is one
library crate; it does not install a query command. It requires Rust 1.88 or
newer and is licensed under MIT OR Apache-2.0.

## Use Zirql

```toml
[dependencies]
zirql = "0.1.3"
```

The [setup guide](https://zayenz.se/zirql/guides/getting-started/) explains how to run the examples from
an extracted crate or a `v0.1.3` checkout. From that directory:

```sh
cargo run --locked --example words
cargo run --locked --example csv -- '
let selected = [p for p in rows if p.active == "yes"];
selected | count
' \
  examples/csv/people.csv
```

The word example prints adjacent pairs in Rust's `Debug` format. The CSV command
prints `3` as JSON. The [CSV guide](https://zayenz.se/zirql/guides/csv/) shows the
input tables, queries and results for selections and reports across named files.
[Build the CSV host](https://zayenz.se/zirql/guides/csv-host/) explains the Rust
adapter and how to use it with your own data.

## Language and host APIs

Constructed values include Booleans, checked signed integers, finite floats,
strings, absence, positional arrays, tuples, insertion-ordered dictionaries and
sets, nominal members and sparse keyed arrays. Both `[x*2 for x in [1,2,3] if x>1]` and
`[x*2 | x in [1,2,3] where x>1]` return `[4,6]` through the same reader.
Comprehensions are eager. Sorting
is stable and retains the collection kind. Optional lookup and `default` handle
absence without hiding arithmetic, type or resource errors.

Programs support immutable bindings, typed external parameters, pure named
functions, destructuring, guarded value matches, state loops, fold and scan.
Compatible tail calls reuse their activation. Native queues and identity sets
retain handles through worklist computations; direct rendering and serialization
of those computational values are unsupported. The common reader recognizes
these forms; each host connects its native types, operations and program syntax.
The word executable accepts one expression. CSV accepts immutable `let` bindings
and one final expression, without named function declarations.

| Integration | Public entry points |
| --- | --- |
| Shared expressions | `ValueReaderHost`, `read_value_expression`, `read_value_expression_in_scope`, `ValueQueryHost`, `evaluate_query_value` |
| Typed value trees | `ValueExpression`, `evaluate_value`, `evaluate_value_with_host_validation` |
| Native pipelines and programs | `Host`, `ProgramHost`, `ProjectionHost`, `evaluate`, `evaluate_program` |
| Typed pure functions | `FunctionHost`, `FunctionRequest`, `PreparedFunctions` |
| Runtime contracts | `DeclaredType`, `RuntimeTypeHost`, `admit_value`, `checked_invoke` |
| Shared native operations | Ordering, set algebra, maps, correlated existence, fixed points, capture matches and bounded paths |

Zirql owns generic query grammar; typed callers can also build the same located
representation. A narrow domain-call hook handles signatures that need model
facts. Host callbacks supply domain navigation, predicates,
projections and identity. Native values need no JSON conversion, `Clone`,
`Send`, `Sync` or `'static` bound to pass through a program.

Use the [language reference](https://zayenz.se/zirql/guides/language-reference/) for syntax and limits,
the [implementation reference](https://zayenz.se/zirql/guides/implementation-reference/) for adapter
choices, and the [Rust API reference](https://docs.rs/zirql/0.1.3/zirql/) for
signatures. [Function and control-flow rules](https://zayenz.se/zirql/guides/functions-and-control-flow/)
describe the shared forms and host requirements.

## Limits and failures

One mutable `Budget` covers parsing, validation, evaluation and nested host work.
The host chooses work, aggregate result, stream-size and nesting allowances and
charges its own scans, copies and rendering. The core supplies no default request
limits. Exhaustion returns a located error without truncating a result.

Unknown native predicate facts remain distinct from absence and errors. Pure
expressions and functions cannot emit or edit. CLI hosts must buffer output
until success when they promise complete output; library callbacks and effects
follow the host's transaction policy. See [diagnostics and
limits](https://zayenz.se/zirql/guides/diagnostics-and-limits/) and [host integration](https://zayenz.se/zirql/guides/host-integration/).

## A borrowed source query

This host accepts `prefix("text")` leaves and returns borrowed words. The host
chooses the initial selection and renders the result after evaluation.

```rust
use std::ops::Range;
use zirql::{
    Budget, EvaluationError, Host, Limits, Located, Location, NativeInput, NativeValue,
    QueryError, Truth, evaluate, parse_pipeline,
};

struct Words;

impl<'a> Host<&'a str, (), Located<String>> for Words {
    type Error = QueryError;
    type Key = ();

    fn evaluate_leaf(
        &self,
        prefix: &Located<String>,
        word: &&'a str,
        _context: &mut (),
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<QueryError>> {
        Ok(if word.starts_with(&prefix.value) {
            Truth::True
        } else {
            Truth::False
        })
    }
}

let query = r#"filter(prefix("a")) | head(1) | count"#;
let pipeline = parse_pipeline(query, |reader| {
    let name = reader.word()?;
    if name.value != "prefix" {
        return Err(QueryError::query(name.range, "expected prefix"));
    }
    reader.expect('(')?;
    let prefix = reader.quoted_string()?;
    reader.expect(')')?;
    Ok(prefix)
}, 8)?;

let source = String::from("ash apple birch apple");
let mut budget = Budget::new(Limits { work: 100, items: 10, nesting: 8 });
let input = NativeInput::collect(
    (), source.split_whitespace(), &mut budget, Location::Source(0..source.len()),
)?;
let result = evaluate(&pipeline, &Words, input, &mut budget)?;
assert_eq!(result.value, NativeValue::Count(1));
# Ok::<(), EvaluationError<QueryError>>(())
```

## Release notes and development

[CHANGELOG.md](https://github.com/zayenz/zirql/blob/v0.1.3/CHANGELOG.md) records releases and the earlier prereleases.
[RELEASING.md](https://github.com/zayenz/zirql/blob/v0.1.3/RELEASING.md) describes publication order and checks.
A local package check does not establish that a version is available on crates.io.

```sh
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --locked -- -D warnings
cargo +stable test --workspace --locked
cargo +1.88.0 test --workspace --locked
```
