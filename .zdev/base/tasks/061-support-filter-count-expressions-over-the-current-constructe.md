+++
schema_version = 1
id = "base-061"
key = "constructed-filter-count-current-input"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-047"]
+++
# Support filter/count expressions over the current constructed input

## Outcome

Standard Zirql can filter and count a word stream directly, including inside a summary record, without a host-specific predicate or a comprehension workaround.

## Context

The user requested {"beginning_with_a": filter(starts_with("a")) | count} while reviewing the documentation on 2026-10-15. With the current examples/words.rs this fails at starts_with with expected comma: the scalar helper needs two arguments and the example exposes only a named constructed binding. Base-047 added constructed sort/unique/tally, but not this form. Start with src/reader.rs, src/value.rs, src/parser.rs and examples/words.rs. Reuse the brief's shared-language boundary and existing native pipeline/current-input rules. Base-058 covers Zincite composition and base-060 covers broader host alignment; this task supplies the missing shared constructed behavior and word demonstration.

## Boundaries

- Implement this in the common reader and callable shared evaluator. The host supplies input, not its own string filtering or counting implementation. Preserve native identity, Unknown/completeness policy and existing explicit starts_with(word, prefix) behavior.
- Keep the change to constructed filter/count, the current-input predicate form needed here, and their expression composition. Do not introduce general partial application, closures, a new evaluator or implicit serialization. Consumer migration and deployment remain separate tasks.

## Done when

- [x] With the host supplying ["ash", "apple", "birch", "apple"] as current input, {"beginning_with_a": filter(starts_with("a")) | count} parses and evaluates to a record containing integer 3 through both the common reader and callable evaluation. starts_with("a") tests each current string with the existing case-sensitive prefix semantics.
- [x] The same filter/count operations compose from an explicit constructed collection source and with the existing constructed pipeline stages where result types permit. Filtering preserves order and duplicates; count counts retained items. Empty input and no matches yield zero.
- [x] Each sibling record field starts from the same input; evaluating one field does not consume or replace the input seen by another. Explicit pipeline sources override that input for their stages. Document scope and precedence, and report missing or incompatible input with a located diagnostic.
- [x] Parsing, validation, filtering and counting use the active request limits. Wrong predicate/input types and exhausted budgets fail explicitly. Existing two-argument starts_with, comprehensions and native pipelines retain their behavior.
- [x] Update the words host and documentation to demonstrate the exact summary expression and how an application supplies current input, retaining the words binding for existing examples.

## Validation

- Use a focused public word-summary case with two independent record fields, an explicit-source pipeline and empty/no-match controls. Reuse existing tests for type refusals and work/result limits; add only the missing checks. No syntax matrix, internal call-count tests or timing assertions.
- Run repository-required Rust and documentation checks, git diff --check and zdev check base --format json. Execute the documented words example.

## Result

Added shared constructed current input with literal prefix filtering and count; record fields retain immutable input and explicit sources compose with existing collection stages.

Validation:

- Independent whole review 1/correction 0 passed: common reader, callable nodes, typed facts/preparation/execution, located refusals, limits and native/function compatibility.
- Stable and Rust 1.88 strict Clippy and workspace tests passed (109 integration tests and 8 doctests each); formatting and all eight documented words commands passed.
- Site check/build passed; rendered seven changed routes in desktop light/dark and mobile light, search and keyboard anchor navigation; 19 pages and 876 internal links checked with no errors.
- git diff checks and zdev check passed; exact integrated snapshot unchanged after independent verification.
