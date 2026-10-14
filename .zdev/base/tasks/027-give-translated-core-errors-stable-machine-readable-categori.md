+++
schema_version = 1
id = "base-027"
key = "typed-core-failure-categories"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Give translated core errors stable machine-readable categories

## Outcome

Zincite and Zirium classify the shared errors they translate without matching human-readable error sentences.

## Context

QueryError in src/lib.rs exposes only location and message. Zincite crates/zincite-query/src/pipeline.rs::core_error matches exact work/item limit strings; Zirium crates/zirium/src/query/parser.rs::shared_error matches exact parser messages and replaces unmatched messages with a generic diagnostic. This is verified integration debt rather than a demonstrated evaluation error. A wording change currently alters adapter translation. Start from those actual translation branches and the core error construction sites; retain their existing diagnostic intent.

## Boundaries

- Use the smallest typed error distinction required by the existing translations, with a general fallback for other messages. Avoid a diagnostic registry, numeric-code service or new result wrapper.
- Keep human-readable messages and original locations, and retain typed host errors in EvaluationError.
- Migrate the affected Zirium and Zincite adapters through their existing ownership coordination and Git dependency policy; do not publish a prerelease.

## Done when

- [x] Core errors that existing adapters distinguish have stable typed categories available through the public API.
- [x] The affected adapters translate those categories without inspecting message text, while preserving existing public diagnostic intent and ranges.
- [x] Focused behavior checks prove classification is independent of message wording and that unclassified diagnostics remain informative.

## Validation

- Reuse existing core budget/parser and consumer diagnostic controls; add only the coverage needed to prove category-based translation.
- Run required core and affected consumer checks and document the public error contract.

## Result

Added public typed core failure categories and migrated Zincite and Zirium translations while preserving original coordinates, native errors and informative fallback messages. Core cef5216; Zincite 4b886c8; Zirium c467e7d.

Validation:

- Independent core and whole-task reviews passed with zero findings; required core and consumer Rust, doctest, Python and documentation checks passed.
- Focused controls prove wording-independent classification and informative fallback; Zirium CI 38501456278 passed all 12 jobs on c467e7dc6cfe9f8f5a1afe2eeb15b46ab2788d69.
