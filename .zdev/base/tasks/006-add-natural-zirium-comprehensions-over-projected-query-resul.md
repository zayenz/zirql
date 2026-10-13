+++
schema_version = 1
id = "base-006"
key = "zirium-comprehension-reports"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-005"]
+++
# Add natural Zirium comprehensions over projected query results

## Outcome

Zirium's JSON-shaped comprehension syntax constructs reports using exactly the same value and generator evaluator as Zincite.

## Context

Read background/comprehensions.md, the implemented shared expression API, Zirium parser/literal/report syntax and current immutable bindings. Implement the proposed [expression for binder in source if condition] form inside Zirium's existing structured-value frontend; repeated for clauses express dependent sources. The brief's equivalent labels report is the cross-host control. Preserve existing plain JSON-shaped literals and interpolation.

## Boundaries

- This adds a frontend, not another comprehension evaluator. Existing bare JSON-shaped values keep their meaning.
- New for/if syntax is recognized contextually inside comprehensions; do not reserve identifiers globally that existing programs could bind.
- Only explicit scalar/structured projections enter scalar comprehensions; native MLIR handles keep their own identity and result path.

## Done when

- [x] The proposed syntax parses and lowers located head/source/filter expressions and dependent generators to shared Zirql nodes.
- [x] Existing binding, literal, interpolation and JSON output programs remain compatible; record keys and bound string values remain distinct.
- [x] Equivalent MiniZinc and Zirium forms produce the same ordered native constructed values before their host-specific renderers.
- [x] Scope, numeric refusals, short-circuit execution and aggregate expansion limits are shared; surface diagnostics point to the original Zirium query.
- [x] Documentation demonstrates one complete query-to-comprehension-to-JSON report and clearly states its supported operations.

## Validation

- Reuse the shared positional controls through both frontends and one existing legacy-literal/interpolation regression control.
- Run Zirql and Zirium required Rust checks and affected Python query tests; avoid duplicate test implementations for the evaluator.

## Result

Zirium now lowers contextual JSON-shaped comprehensions to the shared Zirql alpha.4 evaluator, retains ordered native constructed reports, and carries one allowance through evaluation and CLI rendering. Legacy literals, bindable for/if names and typed MLIR results remain compatible. Consumer source commit: d7869d3b8b74a727bdd2f8a03865724b7ffe3b90.

Validation:

- Independent whole-task v3 review passed all five completion conditions and the rendering, unary-sign and contextual-subtraction corrections; no findings or repository validation writes.
- Simple and dependent reports through actual Zincite and Zirium APIs produced equal ordered native constructed values before either renderer; host JSON/tagged output checked separately.
- Zirql checks and Zirium Rust 1.88/stable tests, Clippy with warnings denied, formatting, doctests and documentation passed.
- Rebuilt CPython 3.11 through 3.14 suites passed with 165 tests and two existing skips each. Fresh macOS arm64 cp314 wheel and sdist build, metadata checks, installations and native query smokes passed.
- Clean cargo package -p zirium --locked passed after the reviewed consumer commit. The normalized package compiles against exact registry alpha.4, its checksum matches the verified publication, and all 43 production sources match the reviewed candidate.
