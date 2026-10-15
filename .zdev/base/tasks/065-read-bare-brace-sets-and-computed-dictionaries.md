+++
schema_version = 1
id = "base-065"
key = "common-brace-collection-reader"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = ["base-062", "base-064", "base-068"]
+++
# Read bare-brace sets and computed dictionaries

## Outcome

The common expression reader lowers short bare-brace set/dictionary literals and comprehensions into the shared evaluator.

## Context

Read background/collection-language-contract.md collection grammar and brief comprehension rules after the shared ordered collection slice. Existing reader record fields are static labels; Set and Set comprehensions already have runtime nodes, and Record already represents string dictionaries. Read src/reader.rs, src/value.rs and src/value/execution.rs and their preparation/name/type/disposal paths. Colon at the brace body level chooses string dictionary entries, otherwise a nonempty brace is a set; {} is an empty dictionary. Explicit control/function braces continue to delimit blocks. Consumer integration must inspect Zirium crates/zirium/src/query/parser.rs::shared_value dispatch/admission and Zincite program/composition.rs::lower_composed_value -> functions.rs::lower_function_value/CST lowering; neither route automatically exposes every new shared form after a dependency change.

## Boundaries

- Shared reader/runtime expression path only; no independent evaluator, arbitrary-key dictionary, string sets, generic parser configuration or host model resolver. Consumer adaptation is limited to demonstrated glue needed to expose common syntax; preserve natural MiniZinc alternatives.

## Done when

- [x] Common text and typed APIs agree for {3,1,3}, set comprehensions, quoted string dictionaries and computed string-key literal/comprehension forms. {k:v} reads k as an expression; no ordinary set/keyed constructor prefixes or new record/dictionary spelling split.
- [x] Exceptional set<int>(), set<bool>() and set<float>() construct typed empty sets; typed empty nominal sets can be supplied by hosts without common model/name resolution. Existing supported set element kinds remain unchanged.
- [x] Use the shared ordered construction/evaluation for dynamic keys, scope, duplicate-before-value failure, known-Boolean filters and first successful generation order. Static dictionary schemas and field/computed access continue to work without a new generalized value type.
- [x] Bare-brace syntax composes with existing arrays, tuples, explicit blocks, grouping and pipelines. Required preparation/free-name/type/copy/disposal paths retain byte ranges, native owners and the active work/result/nesting budget.
- [x] Focused reader/evaluator controls demonstrate one dependent filtered collection, block disambiguation, duplicate key refusing before its erroneous value, unsupported key/type/scope refusal and bounded expansion. Update selected guide sections to executable support actually delivered.
- [x] Both tools adopt the delivered shared revision through actual pinned Git dependencies. Zirium exposes the selected brace forms in its existing inline/file/library entry points with necessary dispatch/admission glue; Zincite exposes corresponding permitted natural MiniZinc forms through shared construction/evaluation. Reuse the compact controls above to prove actual consumer paths, including callable supplied nominal values where relevant; no second parser or broader model/data-edit semantics.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.

## Result

Read brace sets and computed dictionaries through the shared evaluator, with strict string-key access and actual frontend adoption in both hosts.

Validation:

- PASS formatting, locked workspace tests, default and Rust 1.99 Clippy with warnings denied in all three repositories; core Rust 1.88 tests and full Zirium compatibility/MSRV/all-feature/doctest/warning-clean documentation checks.
- PASS shared text/typed/native collection controls, computed strict access and declared children, duplicate-before-value failure, library/inline/file agreement, both Zincite commands and later-error stdout buffering.
- PASS independently reviewed whole candidate and focused dictionary-value-selector correction at snapshot W557f617e5b91157e; all three frozen repository guards remained equal.
- PASS all three sites, actual changed guide commands and links; seven canonical pages visually checked at desktop 1280 and mobile 390 widths.
- Both consumers resolve real pinned Git revision 9293f514c6d20054932c3327ea6c530bcb4a479f; no path override or package publication.
- Zirium integration commit 35dabe45a0dbf3a881568dd818d109b41b4a5569; Zincite integration commit e06140135f45dc0dfc1ad3afb815d046ff028cd3.
