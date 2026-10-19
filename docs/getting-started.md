# Set up the word model

The word-analysis example is a Rust application that supplies an array of strings
as current input and as the binding `words`. It is the host: the application
that embeds Zirql and supplies its data. Zirql reads and evaluates the
expression; the host prints the result.

This guide connects the [word tutorial](examples.md) to its Rust implementation.
The two small trait implementations in the example form its adapter to Zirql.

## Run the example

Use Rust 1.88 or newer. The published crate includes both example sources and
their CSV fixtures. Extract `zirql-0.1.1.crate` and run commands from its
`zirql-0.1.1` directory, or check out the matching release:

```sh
git clone --branch v0.1.1 --depth 1 https://github.com/zayenz/zirql.git
cd zirql
```

From either directory:

```sh
cargo run --locked --example words
```

The default expression finds adjacent word pairs in `ash apple birch apple`.
It returns `["ash apple", "apple birch", "birch apple"]`, printed in Rust's `Debug`
format.

Pass another expression as the first argument:

```sh
cargo run --locked --example words -- '[w for w in words if starts_with(w, "a")]'
cargo run --locked --example words -- '[w for w in words if starts_with(w, "a")] | sort | unique'
cargo run --locked --example words -- '[w for w in words if starts_with(w, "a")] | tally'
```

The three commands return `["ash", "apple", "apple"]`, `["apple", "ash"]` and the dictionary
`{"ash": 1, "apple": 2}`, respectively.

The tally returns fields in first-seen order: `ash: 1`, then `apple: 2`.
The development checkout also supports [tallying unordered neighbouring word
sets](examples.md#count-unordered-neighbouring-word-pairs), with an exact command
and printed output. That addition is not in the published `0.1.1` archive.

To order a dictionary explicitly, sort its keys or select its values:

```sh
cargo run --locked --example words -- 'sort({"z": 2, "a": 2, "m": 1})'
cargo run --locked --example words -- 'sort_by({"z": 2, "a": 2, "m": 1}, value)'
```

These retain dictionaries, ordered `a, m, z` and `m, z, a` respectively.

Build a summary directly from the current input:

```sh
cargo run --locked --example words -- '{"beginning_with_a": filter(starts_with("a")) | count}'
cargo run --locked --example words -- '{"beginning_with_a": filter(starts_with("a")) | count, "total": count}'
cargo run --locked --example words -- '{"explicit": ["apple", "ash", "birch", "apple"] | filter(starts_with("a")) | sort | unique | count, "total": count}'
```

The results contain counts `3`, then `3` and `4`, then `2` and `4`.
Each dictionary field starts from the same input. The explicit array supplies only
its own pipeline; it does not replace the input used by `total`.

Array keys and tuple positions are also available through the common reader:

```sh
cargo run --locked --example words -- '[{"position": i, "word": (words[i], i).1} for i in index_set(words)]'
```

This returns one dictionary per word, with positions `1` through `4`. The default
adjacent-pair expression also uses `index_set(words)`, keeping only keys with
a following word.

The [complete source](../examples/words.rs) is small enough to adapt directly.
This is a repository example executable; the library itself does not install
a standalone query command.

## Supply the input

The application calls Rust's `split_whitespace` and converts each slice to a
`ConstructedValue::String`. It stores those strings in a `ConstructedValue::Array`.
There is no normalization, punctuation removal or case folding.

The host exposes the array under the name `words` and supplies it as immutable
current input. It also supplies type facts so Zirql can check the expression
before running it. The reference calls this check preflight validation. Zirql
owns filtering, counting, sorting, stable unique and tally.

## Read and evaluate an expression

The example implements two interfaces:

- `ValueReaderHost` identifies `words` as a binding and declares that this host
  has no native stages or domain predicates.
- `ValueQueryHost` supplies the binding's value, current-input facts through
  `current_input_types`, and the input through `read_current_input`. Zirql
  retains one admitted input per invocation. The host copies constructed data
  through `copy_constructed_value`, which accounts for the copy in the request
  budget. Native queries are rejected because this model does not define them.

The application calls `read_value_expression`, checks that the reader consumed
the whole expression, then calls `evaluate_query_value`. It passes the same
`Budget` through parsing, type discovery, binding reads and evaluation. This
budget tracks work and result allocations against the host's chosen limits. A
failed parse or evaluation returns a `QueryError` with its source or query
location.

After evaluation, `dispose_native_value` releases the expression tree. The
returned `ConstructedValue` belongs to the caller. The example prints it with
`Debug`; an application can inspect the dictionaries or choose its own output format.

The explicit limits in the example are tutorial choices. Set limits that fit
your application's input and expected queries. Include host scanning, allocation
and rendering in that accounting; see [budget and diagnostic APIs](runtime-limits.md).

<span id="add-a-pinned-dependency"></span>

## Use Zirql in another Rust project

Add the 0.1.1 registry dependency to your manifest:

```toml
[dependencies]
zirql = "0.1.1"
```

Or use Cargo to add the same requirement:

```sh
cargo add zirql@0.1.1
```

These guides describe Zirql 0.1.1. The earlier `0.1.0-alpha.5` registry package
has a smaller language and API. Use the matching
[Rust API reference](https://docs.rs/zirql/0.1.1/zirql/), or generate documentation
for the dependency selected in your lockfile:

```sh
cargo doc --open
```

To work with files and named tables, [query CSV files](csv.md), then follow
[Build the CSV host](csv-host.md). That tutorial uses the same value interfaces
and adds header types, saved bindings and buffered JSON output. The
[implementation reference](implementation-reference.md) covers other data
models and the native interfaces.
