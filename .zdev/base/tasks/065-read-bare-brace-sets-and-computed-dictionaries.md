+++
schema_version = 1
id = "base-065"
key = "common-brace-collection-reader"
area = "base"
status = "open"
complexity = "advanced"
afk = true
blocked_by = ["base-062", "base-064"]
+++
# Read bare-brace sets and computed dictionaries

## Outcome

The common expression reader lowers short bare-brace set/dictionary literals and comprehensions into the shared evaluator.

## Context

Read docs/values.md selected collection grammar and brief comprehension rules after the shared ordered collection slice. Existing reader record fields are static labels; Set and Set comprehensions already have runtime nodes, and Record already represents string dictionaries. Read src/reader.rs, src/value.rs and src/value/execution.rs and their preparation/name/type/disposal paths. Colon at the brace body level chooses string dictionary entries, otherwise a nonempty brace is a set; {} is an empty dictionary. Explicit control/function braces continue to delimit blocks. Consumer integration must inspect Zirium crates/zirium/src/query/parser.rs::shared_value dispatch/admission and Zincite program/composition.rs::lower_composed_value -> functions.rs::lower_function_value/CST lowering; neither route automatically exposes every new shared form after a dependency change.

## Boundaries

- Shared reader/runtime expression path only; no independent evaluator, arbitrary-key dictionary, string sets, generic parser configuration or host model resolver. Consumer adaptation is limited to demonstrated glue needed to expose common syntax; preserve natural MiniZinc alternatives.

## Done when

- [ ] Common text and typed APIs agree for {3,1,3}, set comprehensions, quoted string dictionaries and computed string-key literal/comprehension forms. {k:v} reads k as an expression; no ordinary set/keyed constructor prefixes or new record/dictionary spelling split.
- [ ] Exceptional set<int>(), set<bool>() and set<float>() construct typed empty sets; typed empty nominal sets can be supplied by hosts without common model/name resolution. Existing supported set element kinds remain unchanged.
- [ ] Use the shared ordered construction/evaluation for dynamic keys, scope, duplicate-before-value failure, known-Boolean filters and first successful generation order. Static dictionary schemas and field/computed access continue to work without a new generalized value type.
- [ ] Bare-brace syntax composes with existing arrays, tuples, explicit blocks, grouping and pipelines. Required preparation/free-name/type/copy/disposal paths retain byte ranges, native owners and the active work/result/nesting budget.
- [ ] Focused reader/evaluator controls demonstrate one dependent filtered collection, block disambiguation, duplicate key refusing before its erroneous value, unsupported key/type/scope refusal and bounded expansion. Update selected guide sections to executable support actually delivered.
- [ ] Both tools adopt the delivered shared revision through actual pinned Git dependencies. Zirium exposes the selected brace forms in its existing inline/file/library entry points with necessary dispatch/admission glue; Zincite exposes corresponding permitted natural MiniZinc forms through shared construction/evaluation. Reuse the compact controls above to prove actual consumer paths, including callable supplied nominal values where relevant; no second parser or broader model/data-edit semantics.

## Validation

- Run git diff --check and zdev check base --format json in the authoritative Zirql checkout; run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace for changed Rust repositories.
- Run each affected consumer required checks and focused inline/file/library behavior, including its established later-error output buffering. Use actual pinned Git dependencies and separate framework/consumer commits; no registry publication.
- Update changed guides to distinguish actual support from remaining selected design, and run their established documentation/site checks. Add only compact public-behavior coverage justified by the named change; no callback-count tests, syntax matrices or timing assertions.
