+++
schema_version = 1
id = "base-134"
key = "equality-binding"
area = "base"
status = "open"
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

- [ ] Bare scalar and saved-name equality expressions behave like their report-prefixed forms.
- [ ] Legacy name = expression bindings and reserved-name diagnostics still work.
- [ ] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Add a focused control to the existing public parser/program tests for equality versus assignment; reuse existing binding controls.
