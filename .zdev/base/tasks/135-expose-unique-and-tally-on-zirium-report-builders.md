+++
schema_version = 1
id = "base-135"
key = "report-builders"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Expose unique and tally on Zirium report builders

## Outcome

Rust and Python ReportExpr builders expose the existing unique and tally operations.

## Context

Textual unique/tally and native builders already work. The ReportExpr builders in crates/zirium/src/query/builder.rs and crates/zirium-python/src/query.rs lack these methods, as do the stubs in python/zirium/query.pyi. Use the existing AST and metadata-preserving builder pattern.

## Boundaries

- Do not add new evaluators or general set/dictionary builder constructors.

## Done when

- [ ] Rust and Python ReportExpr.unique() and ReportExpr.tally() execute the same operations as text.
- [ ] Bindings and stubs agree and retain parameter/function metadata.
- [ ] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Run the affected existing Python build, tests, Ruff and ty checks; add only a small public builder control for the missing methods.
