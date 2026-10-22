# Set up the word model

The word-analysis example is a Rust application that supplies an array of strings
as current input and as the binding `words`. It is the host: the application
that embeds Zirql and supplies its data. Zirql reads and evaluates the
program; the host prints the result.

This guide connects the [word tutorial](examples.md) to its Rust implementation.
`ConstructedBinding` and `ConstructedProgram` provide its data and program boundary.

## Run the example

Use Rust 1.88 or newer and run from the Zirql 0.3.0 checkout or extracted crate.
Both include the word and CSV examples.

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
The word example also supports [tallying unordered neighbouring word
sets](examples.md#count-unordered-neighbouring-word-pairs), with an exact command
and printed output.

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
adjacent-pair expression uses `windows(2)` to visit neighbouring words.

The [complete source](../examples/words.rs) is small enough to adapt directly.
This is a repository example executable; the library itself does not install
a standalone query command.

## Supply the input

The application calls Rust's `split_whitespace` and converts each slice to a
`ConstructedValue::String`. It stores those strings in a `ConstructedValue::Array`.
There is no normalization, punctuation removal or case folding.

The application gives that array and a positional string-sequence declaration
to `ConstructedBinding::new`. Admission checks actual data and retains compact
facts from the declaration, including the element type for an empty array. The
same immutable binding supplies the name `words` and optional current input.
Zirql owns filtering, counting, sorting, stable unique and tally.

## Read and evaluate a program

The example calls `ConstructedProgram::read` with the source and initial name
`words`, then evaluates it with the borrowed binding and current input:

```rust
let program = ConstructedProgram::read(&query, &["words"], &mut budget, 64)?;
let result = program.evaluate(
    &[("words", &words)],
    Some(&words),
    ParameterInputs::Cli(&[]),
    FunctionLimits { active_depth: 32 },
    &mut budget,
)?;
```

The same `Budget` covers splitting, allocation, admission, reading, preparation
and execution. The program may contain typed parameters, named functions,
ordered pattern lets and one final expression. The word executable supplies no
external parameter values, so a program requiring them receives a missing-value
error. Function arguments are explicit; bodies cannot capture `words` or current
input. The [CSV example](csv.md) also collects repeated `--param` arguments.

`ConstructedProgram` disposes its parsed trees when dropped. The returned
`ConstructedValue` belongs to the caller, and failures retain query locations
and function-call frames. Words prints with `Debug`; an application can inspect
the records or call a [shared renderer](library-rendering.md).

The explicit limits in the example are tutorial choices. Set limits that fit
your application's input and expected queries. Include host scanning, allocation
and rendering in that accounting; see [budget and diagnostic APIs](runtime-limits.md).

<span id="add-a-pinned-dependency"></span>

## Use Zirql in another Rust project

Add the 0.3.0 registry dependency to your manifest:

```toml
[dependencies]
zirql = "0.3.0"
```

Or use Cargo to add the same requirement:

```sh
cargo add zirql@0.3.0
```

The registry API reference describes Zirql 0.3.0. The earlier `0.1.0-alpha.5` registry package
has a smaller language and API. Use the matching
[Rust API reference](https://docs.rs/zirql/0.3.0/zirql/), or generate documentation
for the dependency selected in your lockfile. For local development, you can
depend on a matching checkout:

```toml
[dependencies]
zirql = { path = "../zirql" }
```

Generate its local API documentation with:

```sh
cargo doc --open
```

To work with files and named tables, [query CSV files](csv.md), then follow
[Build the CSV host](csv-host.md). That tutorial uses the same constructed-program API
and adds header declarations, named tables, parameters and buffered JSON/Markdown output. The
[implementation reference](implementation-reference.md) covers other data
models and the native interfaces.
