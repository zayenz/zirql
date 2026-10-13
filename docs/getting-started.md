# Getting started

To query an MLIR or MiniZinc file from a terminal, use
[Zirium](https://github.com/zayenz/zirium) or
[Zincite](https://zayenz.se/zincite/). For a Rust integration, begin with a
constructed value. The example below needs only a Cargo project.

## Add a pinned dependency

Development uses pinned Git revisions. This revision contains the native
program and constructed-value APIs used below:

```toml
[dependencies]
zirql = { git = "https://github.com/zayenz/zirql", rev = "3a63bf53cd4aff7a70d611c4d150a614e86968e6" }
```

Use a current stable Rust toolchain. Keep the revision pin when integrating a
host, and review API changes before moving it. Usable alpha packages also exist
on crates.io, but they may describe a different revision. The yanked
`0.0.0-prerelease` name reservation is not an installation target.

## Evaluate a small expression

Put this in `src/main.rs` in a Cargo project with the dependency above:

```rust
use zirql::{
    Bindings, Budget, ConstructedValue, Located, ProgramLimits,
    ValueBinaryOperator, ValueExpressionKind, evaluate_value,
};

fn main() -> Result<(), zirql::QueryError> {
    // A frontend would retain the byte ranges from the text "6 * 7".
    let expression = Located::new(
        ValueExpressionKind::Binary {
            operator: ValueBinaryOperator::Multiply,
            left: Box::new(Located::new(
                ValueExpressionKind::Literal(ConstructedValue::Integer(6)), 0..1,
            )),
            right: Box::new(Located::new(
                ValueExpressionKind::Literal(ConstructedValue::Integer(7)), 4..5,
            )),
        },
        0..5,
    );
    let mut budget = Budget::for_program(ProgramLimits {
        work: 1_000, results: 100, nesting: 16, stream_items: 100,
    });
    let value = evaluate_value(&expression, &Bindings::default(), &mut budget)?;
    assert_eq!(value, ConstructedValue::Integer(42));
    println!("{value:?}");
    Ok(())
}
```

Run `cargo run`. The result is `Integer(42)`. The limits here are small-example
choices, not library defaults. `Debug` prints the returned Rust value.

The example constructs the expression tree directly in Rust. A text frontend
would parse `6 * 7` into the same representation and retain its source ranges.
The [common pipeline reader](shared-concepts.md#the-common-text-reader) accepts
the smaller pipeline syntax described in the next guide.

## Read API documentation for your revision

```sh
cargo doc --open
```

The [published API documentation](https://docs.rs/zirql) is useful for looking up
names, but check its version against your dependency. Continue with
[values and comprehensions](values.md) for collection construction, or
[host integration](host-integration.md) to query native domain items.
