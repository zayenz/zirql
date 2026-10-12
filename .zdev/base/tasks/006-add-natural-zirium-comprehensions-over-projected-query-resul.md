+++
schema_version = 1
id = "base-006"
key = "zirium-comprehension-reports"
area = "base"
status = "open"
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

- [ ] The proposed syntax parses and lowers located head/source/filter expressions and dependent generators to shared Zirql nodes.
- [ ] Existing binding, literal, interpolation and JSON output programs remain compatible; record keys and bound string values remain distinct.
- [ ] Equivalent MiniZinc and Zirium forms produce the same ordered native constructed values before their host-specific renderers.
- [ ] Scope, numeric refusals, short-circuit execution and aggregate expansion limits are shared; surface diagnostics point to the original Zirium query.
- [ ] Documentation demonstrates one complete query-to-comprehension-to-JSON report and clearly states its supported operations.

## Validation

- Reuse the shared positional controls through both frontends and one existing legacy-literal/interpolation regression control.
- Run Zirql and Zirium required Rust checks and affected Python query tests; avoid duplicate test implementations for the evaluator.
