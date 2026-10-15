# Set up the word model

The word-analysis example is a Rust application with one binding, `words`,
containing an array of strings. It is the host: the application that embeds
Zirql and supplies its data. Zirql reads and evaluates the expression; the host
prints the result.

This guide connects the [word tutorial](examples.md) to its Rust implementation.
The two small trait implementations in the example form its adapter to Zirql.

## Run the example

Use Rust 1.88 or newer and a development checkout of Zirql. From its root:

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

Array keys and tuple positions are also available through the common reader:

```sh
cargo run --locked --example words -- '[{"position": i, "word": (words[i], i).1} for i in index_set(words)]'
```

This returns one record per word, with positions `1` through `4`. The default
adjacent-pair expression also uses `index_set(words)`, keeping only keys with
a following word.

The first three commands return `["ash", "apple", "apple"]`, `["apple", "ash"]` and the record
`{"apple": 2, "ash": 1}`, respectively.
The [complete source](../examples/words.rs) is small enough to adapt directly.
This is a repository example executable; the library itself does not install
a standalone query command.

## Supply the input

The application calls Rust's `split_whitespace` and converts each slice to a
`ConstructedValue::String`. It stores those strings in a `ConstructedValue::Array`.
There is no normalization, punctuation removal or case folding.

The host exposes the array under the name `words`. It also supplies type facts
so Zirql can check the expression before running it. The reference calls this
check preflight validation. All expressions on the [front page](index.md) work
with this one binding. The shared language reads and evaluates selection,
sorting, stable unique and tally.

## Read and evaluate an expression

The example implements two interfaces:

- `ValueReaderHost` identifies `words` as a binding and declares that this host
  has no native stages or domain predicates.
- `ValueQueryHost` supplies the binding's value. It copies constructed data
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
`Debug`; an application can inspect the records or choose its own output format.

The explicit limits in the example are tutorial choices. Set limits that fit
your application's input and expected queries. Include host scanning, allocation
and rendering in that accounting; see [budget and diagnostic APIs](runtime-limits.md).

<span id="add-a-pinned-dependency"></span>

## Use Zirql in another Rust project

Development integrations use a pinned Git revision. Select and record the
reviewed commit you are integrating:

```sh
cargo add zirql --git https://github.com/zayenz/zirql --rev <reviewed-commit>
```

Replace `<reviewed-commit>` with the full commit hash. Keep the resulting pin
when updating other dependencies. These docs and the word example target the
development checkout. The published `0.1.0-alpha.5` package has an earlier
language subset, including fewer string and ordering helpers. Generate API
documentation for the revision you actually use:

```sh
cargo doc --open
```

[docs.rs](https://docs.rs/zirql) describes published versions. For a larger
integration, continue with the [implementation reference](implementation-reference.md).
