+++
schema_version = 1
id = "base-075"
key = "scalar-result-admission"
area = "base"
status = "done"
complexity = "standard"
afk = true
blocked_by = ["base-074"]
+++
# Accept shared scalar expressions as Zirium query results

## Outcome

Zirium accepts supported shared scalar expressions directly at result positions, consistently with let initializers and the other value frontends.

## Context

The same scalar evaluator already works in bindings, but ordinary result admission is narrower. At the base-074 source candidate, let n=1; n+2 fails at the + token while let n=1; let total=n+2; total returns 3. Bare 1+2 also fails in the current canonical executable. Zincite accepts the direct expression and the CSV reader owns the same shared arithmetic. Read Zirium query/parser.rs::pipeline/stage/shared_value, comprehension.rs::starts/has_syntax, and the accepted base-074 native-leaf admission and output-kind paths. This is frontend admission of existing Zirql expressions; the brief owns scope, checked arithmetic, type rules, effects, output and testing. Preserve base-074 native argument recursion repair and constructed suffix routing.

## Boundaries

- Reuse existing shared reading, type validation and evaluation. Do not add operators, coercions, a parser framework, another evaluator, or parser-side type inference. Do not broaden native primitive or collection element domains.
- Preserve native source/stage/pipeline and ordering argument routes, legacy JSON insertion, native identity/completeness, lexical scope, original input, one request budget, located failures and existing host rendering/output conventions. Other hosts need no changes merely for symmetry.
- Keep broader guide rewriting and indexed-array work in their tasks. Preserve unrelated changes; use a real shared Git revision only if an actual core change is required. No package publication or deployment.

## Done when

- [x] Zirium actual library and inline/file CLI paths accept let n=1; n+2 and bare 1+2 as scalar 3, using the same existing shared value semantics as the corresponding initializer. Direct supported string/Boolean scalar expressions also follow that value path without a function declaration or redundant binding.
- [x] Common scalar operators, grouping and postfix/value syntax retain existing precedence, type, checked-arithmetic and location rules. Plain native bindings/stages, saved native selectors, native argument expressions and their output kinds keep their existing routes. Unsupported expressions and incompatible operators still fail before partial CLI output.
- [x] Extend compact existing public controls for direct-versus-initializer agreement, a located scalar failure and later-error buffering. Reuse existing native/argument/output and parsing/preparation budget controls; no syntax matrix, callback counts or timing assertions.
- [x] Current Zirium guides show a directly executable scalar expression and its immutable-binding form. Replay representative CSV/Zincite counterparts as existing behavior without changing their output envelopes or adding consumer code unnecessarily.

## Validation

- Run affected repository formatting, diff checks, default and explicit Rust1.99 Clippy with warnings denied, locked workspace tests and required Zirium stable/1.88 all-target/all-feature compatibility, doctest/rustdoc and Python query checks. Use the brief focused public-behavior level.
- Run exact changed guide commands, affected site check/build/link checks and desktop/mobile review. Root runs authoritative zdev checks and separately attributable source/record commits after independent verification.

## Result

Shared scalar and postfix results agree with initializers; native field pipelines preserve direct, grouped and saved output kinds.

Validation:

- Independent whole review, two retained corrections and focused rechecks; final pass with unchanged snapshot/all-repository guards.
- Required core and Zirium Rust checks, stable/1.88 compatibility, Python 175 passed/2 skipped, actual inline/file CLI and counterpart probes, guide/site/link/desktop/mobile checks passed.
