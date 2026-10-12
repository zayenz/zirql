# Repository guidance

## Keep the framework small

Every abstraction needs a concrete use in Zirium or Zincite. Prefer one Rust
library crate, direct evaluation and typed native results. Check whether each
interface and test earns its maintenance cost.

## Planning and implementation

Read [.zdev/base/brief.md](.zdev/base/brief.md) before planning or implementing
framework work. It owns shared language rules, host boundaries and testing.
Read its linked comprehension contract when working on expressions or data.

Use zdev for durable work in `.zdev/base/`. The base area follows `main` and
planning records are tracked. `TASKS.md` is generated; do not edit it by hand.
Use the selected task's boundaries and completion conditions. Planning does
not authorize implementing the framework or changing a consumer repository.

Zirql must remain independent of both consumer crates. Host adapters retain
native handles and source coordinates; rendering is an explicit boundary.
Surface parsers may differ, but supported comprehensions share one evaluator.
Keep MiniZinc model facts, MLIR dialect semantics and source-edit safety with
their hosts. Preserve unrelated changes in every repository touched by an
approved integration task.

For documentation and metadata, run `git diff --check` and
`zdev check base --format json`. No new product tests are needed.
Once the Rust crate exists, also run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Integration tasks also run the affected host's required checks. Use focused
public-behavior coverage from the brief; avoid tests of internal call counts,
generic test matrices and timing assertions. Track `Cargo.lock` when created.
