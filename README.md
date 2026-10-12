# Zirql

Zirql is a planned Rust framework for building purpose-built query languages.
Its first consumers are [Zirium](https://github.com/zayenz/zirium), for MLIR, and
[Zincite](https://github.com/zayenz/zincite), for MiniZinc.

The framework will share bounded pipeline execution, bindings, structured
values and comprehension evaluation. Hosts will supply their own operations,
predicates, native handles, source rendering and edit policy. Surface syntax
can follow the host language while lowering to the same evaluation rules.

The repository currently contains planning records. No library or command is
implemented, and no crate is published.

Read the [base brief](.zdev/base/brief.md) for the proposed architecture,
integration order and testing level. [TASKS.md](.zdev/base/TASKS.md) is generated
by zdev; tasks enter that index after the reviewed bundle is approved.

The GitHub repository starts private. Making it public is a later decision.

License: MIT OR Apache-2.0.
