+++
schema_version = 1
id = "base-126"
key = "typed-native-function-calls"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-114"]
+++
# Declare ordinary native functions once and share call admission

## Outcome

Both consumers define ordinary eager native functions with one signature and implementation, using Zirql parsing, preparation and argument admission.

## Context

Read brief.md sections Consolidate language machinery and data rendering (2026-10-21) and Complete the common language boundary (2026-10-21), including their compatibility, documentation and validation contracts. Inspect src/functions.rs declarations/prepared calls, src/runtime_types.rs OperationSignature/checked_invoke and src/reader.rs call parsing before introducing a new interface. Zirium query/parser.rs packs same_operation arguments into a synthetic tuple that query/functions.rs unpacks. Zincite program/functions.rs Primitive::named/lower_primitives and program/composition.rs duplicate call parsing, arity and result-kind logic. Reuse the admitted type-fact operations from base-114.

## Boundaries

- Use a small direct host lookup or static definitions. Initial scope is existing eager pure functions; special nominal-identifier grammar, lazy predicates and effectful operators retain their native hooks.
- Signatures do not replace native owner, singleton, completeness, model or source validation. No plugin registry or separate function evaluator.

## Done when

- [ ] One host definition supplies a stable name or ID, parameter and result declarations, and the existing pure/source admission information needed by its callers; shared code parses and prepares ordinary arguments and invokes the implementation with the active budget.
- [ ] Both hosts migrate their existing ordinary eager primitives, including direct_children and same_operation/same_declaration, removing duplicated name recognition, generic argument parsing, synthetic tuple packing and common arity/type dispatch.
- [ ] Callable and textual calls preserve argument order, located errors, native results, statement/function isolation, domain validation and host effect restrictions.
- [ ] Rust API and embedding docs demonstrate defining one native function; host function guides link common calling rules and retain domain-specific signatures and limitations.

## Validation

- Reuse unary and binary primitive controls in both hosts, plus existing arity/type, ownership and incomplete-model refusals; add only an uncovered admission case.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.
