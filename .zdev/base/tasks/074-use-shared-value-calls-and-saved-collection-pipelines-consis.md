+++
schema_version = 1
id = "base-074"
key = "shared-value-dispatch"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = ["base-065"]
+++
# Use shared value calls and saved collection pipelines consistently

## Outcome

CSV and both consumers route supported saved constructed collection pipelines through Zirql, and Zirium accepts existing shared value calls consistently in bindings and final expressions.

## Context

The shared operations exist, but frontend admission differs. With the people CSV fixture, a saved filtered array followed by | count fails unless the binding is parenthesized. Zirium rejects let numbers=[3,1,3]; sum(numbers), while binding its sum first returns 7; its saved numbers | sort | unique | count also fails, whereas Zincite returns 2. These failures persist in the completed brace-reader candidate. Read src/reader.rs::Reader::atom and its suffix/native routing, examples/csv.rs and words.rs ValueReaderHost implementations, and Zirium query/parser.rs::stage/shared_value, comprehension.rs::starts/has_syntax and program.rs native-kind admission. The core and adapter duplicate fixed common-call name lists. Existing shared suffix/evaluation and actual value-kind facts should own the language behavior; preserve native routing when the source is native. The brief owns supported stages, types, scope, effects and testing.

## Boundaries

- Repair existing common-call and supported constructed-suffix dispatch. Do not add a second evaluator, operation registry, parser configuration framework, language operation, numeric coercion or new collection type. Do not widen reducer/stage element domains merely to make dispatch pass.
- Keep actual native source, saved/local/function selectors, native ordering-call argument distinctions and original-input behavior. Unsupported calls and incompatible suffixes remain located errors. Preserve host native owners/completeness, lexical scope, one budget, emission and data-edit policies.
- Keep the CSV example at ordered let bindings plus one final expression. Do not add functions, files/modules, aliases or a full program host to that example. Preserve unrelated work and use actual Git pins without publication or deployment.

## Done when

- [x] The actual CSV executable accepts rows | count and a bare saved filtered array | count with the same results as grouped forms, including named-table bindings. The word example uses the same reader rule. Remove the documentation grouping requirement only for forms now accepted; unsupported native stages still fail.
- [x] Zirium final sum(numbers) returns 7 for [3,1,3], consistently with the same call in a let initializer, without needing an unrelated function declaration. Existing fixed common value-call classification is shared where it removes demonstrated adapter list drift; native names, argument-shape distinctions and declared user functions keep their appropriate routes.
- [x] Zirium accepts let numbers=[3,1,3]; numbers | sort | unique | count and returns 2 through the existing shared constructed operations, matching Zincite and the shared core. Saved results and function/local constructed sources compose through the supported suffix machinery without rendering or a host copy of collection algorithms. Native saved/local/function pipelines retain their existing behavior.
- [x] Compact existing public controls cover the named constructed pipeline, common final-call/initializer agreement, one incompatible suffix with its original range, native ordering and saved/function/local sources, and later-error stdout buffering. Preserve parsing/copy/preparation allowances and native identities; no syntax matrix, callback-count or timing tests.
- [x] Affected consumers adopt the required shared Git revision through real manifest pins and locks, with separately attributable commits. Current guides and CSV examples teach the actual accepted bare forms and common calls. Replay representative core/CSV and both host inline/file/library paths while preserving their output conventions.

## Validation

- Use the brief focused public-behavior testing level. Run required formatting, default and explicit Rust 1.99 Clippy with warnings denied, locked workspace tests and core Rust 1.88 tests; run affected host compatibility checks and actual Cargo dependency metadata.
- Run exact changed guide commands, affected site check/build/link checks and desktop/mobile review. No package publication or deployment. Keep broader reader-writing work in its existing task.

## Result

Share fixed value-call admission and constructed collection suffix lowering; CSV/words bare bindings and Zirium saved/local/function pipelines use the shared evaluator while preserving native routes.

Validation:

- Independent whole review PASS at W4fa5e8c621d6d9c8; native comparison and all three complete Git guards equal; no required corrections.
- All 29 canonical Rust/diff/metadata checks, exact BCF Git identities, Python16 and175 passed with2 existing macOS skips, and37 actual executable probes passed.
- All three site check/build/link checks and six changed guides at desktop/mobile in light/dark themes passed.
- Separate consumer commits: Zirium 972e1d2651b76a6b8d587c1f9a7d6290f0ee8580; Zincite 2c80af397474a13bf8b9d59c9dc6d68e0d52b22a.
