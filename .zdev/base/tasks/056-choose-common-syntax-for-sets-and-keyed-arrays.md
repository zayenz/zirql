+++
schema_version = 1
id = "base-056"
key = "choose-common-set-keyed-syntax"
area = "base"
status = "done"
complexity = "standard"
priority = "normal"
blocked_by = []
+++
# Choose common syntax for sets and keyed arrays

## Outcome

An accepted grammar and small implementation scope make existing shared sets and keyed arrays expressible through the common frontend.

## Context

Zirql already represents and evaluates Set, KeyedArray and KeyedComprehension, but the common reader only constructs records and positional comprehensions. Zincite supplies MiniZinc forms. Read src/value.rs, src/reader.rs, docs/values.md, the brief comprehension contract and Zincite docs/query-comprehension-contract.md plus its comprehension lowering. The accepted Collection syntax and order section of the brief supersedes earlier automatic type/lexical ordering: ordinary sets and dictionaries use bare braces and preserve insertion order; sorting keeps the collection kind. This task records that design and the bounded implementation work it requires.

## Boundaries

- Design and documentation only. Explain disambiguation from records, arrays, blocks and pipelines; retain natural Zincite surface forms where useful. No parser/runtime implementation, speculative value types, string-set expansion, native reflection or broader data-edit policy. Order-preserving dictionary output belongs to the subsequent implementation scope.

## Done when

- [x] Compare at most two concrete syntax options and recommend one using a compact set example and a sparse keyed report with explicit axes. Cover literals and comprehensions, multidimensional keys, typed empty sets/axes, and nominal keys supplied by host values without inventing a shared model resolver.
- [x] The proposed grammar explains disambiguation from records/blocks/arrays, nesting and operator precedence, duplicate-key refusal, and which empty cases require explicit type/axis information. Show lowering into the existing shared nodes and preserve their ordering, identity and budget contracts.
- [x] Present the examples and tradeoffs to the user, record the accepted choice in the base brief and update the affected language documentation. Completion requires user acceptance, followed by a bounded implementation scope ready for subsequent task creation; do not silently choose unresolved syntax or create implementation tasks as part of this design task.

## Validation

- Check each example against the current reader grammar and typed collection contracts; distinguish proposed syntax from executable forms. No new product tests are expected. Run git diff --check and zdev check base --format json and any repository-required documentation checks.

## Result

Selected bare-brace sets and string dictionaries with insertion order and same-kind stable sorting, plus array-shaped indexed syntax and optional declared index sets; recorded bounded implementation gaps.

Validation:

- Independent whole-task review passed with unchanged task snapshot and ownership hashes.
- Formatting, strict Clippy, workspace tests including eight doctests, zdev and diff checks, site check/build and 906 internal links passed; rendered selected-design guide inspected.
