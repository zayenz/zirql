+++
schema_version = 1
id = "base-047"
key = "shared-value-pipelines"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Compose constructed collection pipelines in the shared language

## Outcome

Applications can compose selection, sorting, stable deduplication and string frequency tallies through Zirql common readers and evaluation, including the runnable word host.

## Context

The user requested this high-priority follow-up while reviewing the Zirql blog examples on 2026-10-15. With words ["ash", "apple", "birch", "apple"], selecting words starting with "a" must compose into a sorted distinct list ["apple", "ash"] and a tally apple:2, ash:1. src/parser.rs::parse_pipeline exposes filter/head/count although shared Stage::Unique/Tally and their native evaluator already exist. The common value reader has sort(array), but the constructed-only examples/words.rs cannot pipe a value through these ordinary collection operations. Trace src/reader.rs, src/parser.rs, src/eval.rs and the existing value/prepared execution before choosing the smallest common representation and coherent syntax. Reuse established comparison, ordering and tally semantics; applications supply data and domain operations rather than reparsing or executing generic collection stages. Preserve the separately owned documentation rewrite and use its accepted word example as the baseline.

## Boundaries

- Shared language/library and runnable documentation only. Preserve existing host syntax, native identities/results, terminal-stage rules, diagnostics and limits. No consumer feature migration, package publication, website deployment, plugin system, new optimizer or benchmark framework. Keep unsupported value kinds explicit rather than adding implicit string conversion.

## Done when

- [x] A constructed collection expression composes through shared syntax and a callable typed library path into sorting, stable unique and string tally without a host implementing those generic parsers or evaluators. The small pipeline reader also exposes its existing shared unique/tally stages; preserve existing native APIs and domain extension hooks.
- [x] For the word input in Context, the runnable example demonstrates the ordered selected array, ["apple", "ash"] after sort/unique and a deterministic tally containing apple:2 and ash:1. Empty inputs, duplicate preservation/removal, stable ordering and tally ordering follow documented shared rules.
- [x] Supported scalar types reuse existing comparison/admission and checked count semantics. Incompatible inputs and unsupported kinds fail at the responsible expression/stage; nested use shares normal work/result/item/nesting limits and performs admission before known allocation. Existing comprehensions, sort calls, native pipelines and short queries remain compatible.
- [x] The example and reader-facing guides show complete executable selection/list/statistics expressions and explain the small data/identity/domain boundary. Preserve the prior documentation author changes and verify the updated local guide UI.

## Validation

- Extend a small existing public reader/value/native control set for the motivating pipeline, empty/duplicate/order/type refusals and request-limit failures; compare text and typed results. Do not introduce a syntax matrix, internal call-count tests or timing assertions.
- Run the runnable words example for all three documented forms and check its exact results; run required Rust stable/MSRV formatting, strict Clippy, workspace tests/doctests, git diff --check and zdev check base.
- Run the existing local documentation check/build and verify changed guide examples, navigation/search/links and mobile/light/dark presentation against the frozen candidate. No publication or deployment.

## Result

Shared constructed pipelines compose sort, stable scalar unique and checked string tally through the common reader and typed evaluator. Updated word examples and guides preserve the latest documentation and highlighting.

Validation:

- Independent whole review PASS; stable and Rust 1.88 workspace tests and 8 doctests each, formatting and strict locked Clippy passed.
- Fresh word selection, sorted unique, tally and default outputs matched exactly; site check/build passed with 19 pages and Pagefind.
- Root browser review passed: changed guides, desktop/mobile, light/dark, search, keyboard focus and 522 local links; canonical snapshot and frozen source matched before completion.
