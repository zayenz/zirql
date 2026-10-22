+++
schema_version = 1
id = "base-142"
key = "diff-program-reader"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Use the common program reader for Zirium diff reports

## Outcome

Zirium diff reports share common statement and collection-argument reading and retain their diff semantics.

## Context

Diff execution already uses PreparedProgram, but query/diff_text.rs splits statements and bindings manually and parses head/tail/check counts as literal usize. Ordinary `names | head(1+0) | json` works; diff `before | names | head(1+0) | json` fails. Some constructed expressions, including dictionaries, and legacy bindings already work. Bare positional reports such as [1,2] fail, and support for common lets and named functions remains incomplete in this frontend. Start at diff_text parse_statements and diff_program preparation. The CLI rejects --param in diff mode, and DiffProgram evaluation passes no parameters. Connect both public entry points to common parameter admission.

## Boundaries

- Do not add domain operations, widen effects or implement another evaluator. Common function support extends the frontend; it was not previously supported.

## Done when

- [ ] Diff reading uses read_program/ProgramReaderHost and the shared expression/operand grammar. The generic statement, binding and argument splitting it replaces is removed.
- [ ] Diff reports accept existing common constructed expressions, pattern lets, parameters and named pure functions under ordinary scope rules.
- [ ] The existing callable diff APIs and CLI --param arguments pass declared parameters through common admission and use its ordinary diagnostics for missing, extra and invalid arguments.
- [ ] Computed head/tail/check arguments use common admission and diagnostics.
- [ ] Diff-native stages, before/after ownership, comparison subjects, mutation refusal and statement-end emission remain local and compatible.
- [ ] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Extend existing public diff program tests with a computed count and a small let/function constructed report; reuse side-ownership, mutation and buffered-error controls.
- Exercise the existing small-query preparation benchmark without a new threshold.
- Exercise the public diff CLI with --param and the callable API with a declared parameter used in a computed count. Use the existing admission controls to check missing arguments, extra arguments and invalid argument types.
