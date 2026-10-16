+++
schema_version = 1
id = "base-091"
key = "one-shared-query-reader"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-090"]
+++
# Read both comprehension forms and shared query expressions in Zirql

## Outcome

Both comprehension spellings and the existing generic query-expression forms needed by the consumers use one Zirql reader, located AST and evaluator; host parsing remains domain-specific.

## Context

Read One shared query reader and Shared destructuring patterns in the brief. Consume base-090's actual reviewed binder/pattern API and base-086 scoped arguments. The shared reader currently accepts repeated for/in/if clauses; Zincite mostly parses generic query expressions through its MiniZinc CST, embedding selected common constructs. Root reproduced ordinary Zincite --program [x*2 for x in xs if x>1] failing while the pipe/where form returns [4,6]. Begin with src/reader.rs collection/collection_binders, expression/operator, parenthesized atoms, conditional, indexing, native/value suffix and lexical readers. Read Zincite crates/zincite-query/src/comprehension.rs lowerer and program/functions.rs common_collection_range/function_expression/lower_function_value, composition.rs parse_function_value/lower_composed_value, program.rs pure_value and data.rs callers as concrete grammar requirements; do not edit consumers here. Those routes need existing natural records/conditionals/indexing/literal and lexical forms as well as the generator pipe, or a second generic parser would remain. Host integrations own their subsequent migration and admission policy.

## Boundaries

- Use the existing value/generator/pattern representation, shared scope/type/evaluation machinery and cumulative limits. Add no second evaluator, configurable grammar framework, syntax plugin system, first-class function feature, model execution or new optional expression capability.
- Limit alternate syntax to existing supported Zincite query forms and the user-selected shared patterns. Input MiniZinc CST/literal decoding, MLIR/diff semantics, model/nominal identity, native operations, effects, keyed target validation and serialization remain host-owned. Core remains independent of consumer crates.
- Own core/CSV reader code, focused public controls and current-language/API documentation only. No consumer source edits, registry publication, release tags/assets or site deployment; Root owns source transport, commits and later real dependency pins.

## Done when

- [ ] The public shared reader accepts common for/in/if and MiniZinc pipe/where comprehensions for existing positional, set, dictionary and indexed constructors. Existing grouped in generators, dependent sources, single-value binding generators, guards and base-090 patterns lower to the same existing clauses and ordering/evaluation rules. Do not invent a new common for-pattern-equals syntax.
- [ ] Head binder discovery and sequential source/guard scope work in both forms, including patterns and names colliding with host stage names. Generator pipes are distinguished from grouped native/query pipes and outer suffix pipes; all lookahead and nesting remain bounded with original source ranges.
- [ ] Existing domain-independent MiniZinc query expression forms needed for sole-reader adoption are handled in Zirql: natural records, then/elseif/endif conditionals, Boolean spellings, multidimensional access, supported matrix/starting-key literal forms and quoted-name/string/trivia spellings. Lower them into existing nodes and preserve their established semantics rather than routing whole expressions to a host parser. Unsupported forms receive located refusals.
- [ ] Scoped public expression readers and any narrowly necessary domain atom/call hook let native argument, function, predicate and Data callers use the shared grammar with lexical names. No generic-expression CST parser is required by the documented adapter call sequence; host contextual AST admission and native/nominal/keyed evidence remain explicit.
- [ ] A few focused public reader programs show paired spellings returning [4,6], grouped/dependent/binding order, one destructured binder per notation, a natural record/conditional/indexed head with a collection suffix, and located scope/type/budget refusal. Packaged CSV runs both spellings through one reader; preserve its let-plus-result and output limits.
- [ ] Required stable/MSRV Rust, docs, package and site/link checks pass. Reader/API/language/release guides describe current supported forms and the small domain boundary; Root independently verifies, commits and pushes the actual source for both host integrations.

## Validation

- Run required fmt, fatal workspace/all-target Clippy, workspace tests, Rust 1.88 tests and warning-fatal docs/doctests. Use focused existing-style public grammar/behavior controls; no form-by-position matrix or full MiniZinc conformance campaign.
- Compare paired public-reader programs, generator order and lexical head/source classification, including existing skipped-runtime-error, expansion and empty-keyed-shape contracts. Inspect the concrete Zincite/Zirium call sequences to ensure no generic parser escape hatch is required; consumer tests run in their integration tasks.
- Verify the source package and a few extracted CSV programs in both notations with located refusal and empty stdout. Run affected site check/build/links and Root desktop/mobile/search review. Keep real registry publication separate.
