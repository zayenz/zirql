+++
schema_version = 1
id = "base-134"
key = "equality-binding"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Distinguish equality from legacy assignment in Zirium

## Outcome

Bare equality expressions parse consistently with explicit report expressions.

## Context

In Zirium, query/parser.rs NativeReader::read_binding consumes the first equals sign of ==. `let n=1; n == 1` is misread as a binding and fails, while `let n=1; report n == 1` returns true. Start at read_binding and its existing parser controls.

## Boundaries

- Keep the correction local unless another reader needs the same token helper.

## Done when

- [x] Bare scalar and saved-name equality expressions behave like their report-prefixed forms.
- [x] Legacy name = expression bindings and reserved-name diagnostics still work.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Add a focused control to the existing public parser/program tests for equality versus assignment; reuse existing binding controls.

## Result

Bare equality expressions parse like report expressions. Legacy assignment bindings remain supported.

Validation:

- Independent verification passed: Rust formatting, Clippy, workspace tests, doctests and documentation.
- Site checks, build, links and documented equality example passed. Added prose received Astra medium copyediting.
- Zirium commit 3ad16e8fe7f4b631d3644f3c6dd19caa55298aa0
