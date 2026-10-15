+++
schema_version = 1
id = "base-058"
key = "zincite-composed-value-pipelines"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-047"]
+++
# Compose saved native and constructed results consistently in Zincite

## Outcome

Zincite read-only programs compose intermediate native results, MiniZinc value expressions and shared constructed pipelines through the same bounded Zirql execution.

## Context

The alignment plan in the base brief covers complete programs, not just let spelling. Zincite base-140 separately removes source syntax and unifies input/calculated let bindings; use its integrated committed result before modifying consumer source. Base-047 delivered constructed sort/unique/tally in Zirql and explicitly excluded consumer migration. Inspect Zincite program.rs expression dispatch, program/composition.rs::composition_syntax and ValueReaderHost, comprehension.rs and program/functions.rs. Current routing differs for native, pure and function-bearing requests; the pinned consumer dependency predates 047. Existing native projections, immutable Bind/Binding/Sequence, scalar sort calls and report rendering already exist.

## Boundaries

- Read-only program composition and necessary shared reader/lowering support only. Keep MiniZinc comprehension syntax and existing native syntax; do not implement a second general expression or pipeline evaluator. Do not duplicate base-140 or base-044/046, or broaden data-edit helpers/effects. Coordinate source ownership and preserve their integrated outcomes.
- Keep explicit native-to-constructed projection, nominal/keyed identity, completeness, source coordinates, terminal emitters and whole-request budgets. No implicit JSON/text conversion, extra stage algorithms, new function syntax or package publication.

## Done when

- [x] A program saves a domain selection, derives names and a count from that selection, binds a constructed array/comprehension, then composes sort/unique or tally before reporting. Literal, saved and parenthesized constructed inputs use 047 semantics; MiniZinc generator pipes are distinguished from outer pipeline pipes.
- [x] The same supported expressions have the same meaning with or without an unrelated pure function declaration and in supported function bodies. Saved values are evaluated once and reused without rerunning the source query; nested native queries retain their input and lexical bindings.
- [x] Zincite lowers constructed pipeline stages to shared Zirql representation and evaluation. Any generic suffix-reading/lowering behavior needed by both frontends belongs in Zirql; domain parsing and MiniZinc CST lowering remain with Zincite. Existing scalar-call forms and native streams continue to work.
- [x] Empty inputs, supported scalar ordering/deduplication, string-only tally, terminal-tally restrictions and incompatible transitions follow the existing shared contracts with located refusals. Both Zincite CLI forms and Program library entry points expose the result; update the reviewed Git dependency and reader examples.

## Validation

- Reuse the native-composition/report-ordering fixtures with one saved-selection and constructed-array program, paired with/without an unused function. Check inline/file/library results, a representative bad transition, and later failure leaving stdout empty. Include sparse/keyed refusal or explicit conversion where relevant; no syntax matrix or internal call-count tests.
- Run required checks in every changed repository, including Rust formatting, strict workspace Clippy and tests; run git diff --check and zdev check base --format json. Record separate consumer commits.

## Result

Zincite composes saved native selections, projected names/counts and MiniZinc arrays through shared constructed suffix reading and the existing evaluator. Prepared routing retains known constructed bindings; native ownership, scope and report-only policy remain intact.

Validation:

- Independent whole review1/correction1 PASS after Root-confirmed saved-binding routing correction. Full Wa89fd7e75e65ce3b comparisons and updated two-repository candidate/index/source guards match; no review source writes.
- Fresh canonical framework and Zincite formatting, strict Clippy and workspace tests pass. Focused recheck repeats consumer gates and both-CLI routing/function/refusal/budget/buffering controls; exact actual Git a784e426 dependency and source hashes verified.
- Both production-base site checks/builds pass. Desktop/mobile/anchor/reload/search/keyboard checks and 871 core plus 1055 Zincite internal links pass. The exact saved-selection guide program passes both CLIs inline/file with equal complete results and native source coordinates.
