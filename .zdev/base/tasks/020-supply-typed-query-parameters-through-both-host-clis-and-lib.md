+++
schema_version = 1
id = "base-020"
key = "typed-query-inputs"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-019"]
+++
# Supply typed query parameters through both host CLIs and libraries

## Outcome

The first two example programs run unchanged with typed caller inputs through inline, file and library entry points.

## Context

Read the brief's exact param and --param contract and examples 1/2. Start at src/program.rs Bindings::new, the shared frontend introduced by correlated projection, and both consumer parse/evaluate/CLI boundaries. Primitive predicate arguments must accept checked expression values rather than source substitution.

## Boundaries

- Zirql owns declaration checking, scope, admission and argument validation; hosts transport values and implement domain operations.
- Initial declarations support int, string and bool only. No defaults, imports, functions, named result schemas, undeclared implicit parameters or query-text interpolation.

## Done when

- [ ] param declarations and typed library inputs share the existing immutable binding path; duplicate, missing, extra and wrong-type arguments fail before domain execution.
- [ ] Zirium and both Zincite query commands implement --param name=value decoding exactly as specified in the brief, in inline and file modes.
- [ ] Parameters compose with pure expressions and typed domain predicate/stage arguments, including op(operation_name), under the shared budget and located validation.
- [ ] Examples 1/2 produce their specified results for the supplied inputs. Literal-only short commands still need no parameter declarations; parameterized inline/file/library behavior agrees.

## Validation

- Use the two existing example controls for successful binding and one quoted string treated strictly as data; focused argument refusals cover declaration and CLI decoding boundaries.
- Run required checks in all affected repositories and their focused CLI/library tests; preserve current emission/error behavior.
