+++
schema_version = 1
id = "base-056"
key = "choose-common-set-keyed-syntax"
area = "base"
status = "open"
complexity = "standard"
priority = "normal"
blocked_by = []
+++
# Choose common syntax for sets and keyed arrays

## Outcome

An accepted grammar and small implementation scope make existing shared sets and keyed arrays expressible through the common frontend.

## Context

Zirql already represents and evaluates Set, KeyedArray and KeyedComprehension, but the common reader only constructs records and positional comprehensions. Zincite supplies MiniZinc forms. Read src/value.rs, src/reader.rs, docs/values.md, the brief comprehension contract and Zincite docs/query-comprehension-contract.md plus its comprehension lowering. This task resolves syntax; it does not reopen the established collection semantics.

## Boundaries

- Design and documentation only. Preserve existing record, array, block and pipeline parsing and Zincite surface syntax. No parser/runtime implementation, new value types, string-set support, native reflection, renderer expansion or broader data-edit policy.

## Done when

- [ ] Compare at most two concrete syntax options and recommend one using a compact set example and a sparse keyed report with explicit axes. Cover literals and comprehensions, multidimensional keys, typed empty sets/axes, and nominal keys supplied by host values without inventing a shared model resolver.
- [ ] The proposed grammar explains disambiguation from records/blocks/arrays, nesting and operator precedence, duplicate-key refusal, and which empty cases require explicit type/axis information. Show lowering into the existing shared nodes and preserve their ordering, identity and budget contracts.
- [ ] Present the examples and tradeoffs to the user, record the accepted choice in the base brief and update the affected language documentation. Completion requires user acceptance, followed by a bounded implementation scope ready for subsequent task creation; do not silently choose unresolved syntax or create implementation tasks as part of this design task.

## Validation

- Check each example against the current reader grammar and typed collection contracts; distinguish proposed syntax from executable forms. No new product tests are expected. Run git diff --check and zdev check base --format json and any repository-required documentation checks.
