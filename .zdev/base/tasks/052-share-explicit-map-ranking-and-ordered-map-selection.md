+++
schema_version = 1
id = "base-052"
key = "shared-map-ranking"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-051", "base-048"]
+++
# Share explicit map ranking and ordered map selection

## Outcome

Zirql can explicitly rank map entries by scalar value and preserve that order through selection and host rendering.

## Context

Zirium query.rs::sort_map_by_value and RankedMap implement sort_by(value), followed by reverse/head/tail, only in the host. Read query-language.md Ordering, bounds, and extrema. These legacy scalar rules differ from constructed sort: null, Boolean, numeric and string map values are supported; mixed integer/float numbers compare without losing integer precision. Use the shared map construction/result boundary from the preceding map task.

## Boundaries

- Preserve existing host syntax, public result types, domain identity/order, diagnostics, effects and limits. Keep domain operations and rendering in the host; do not add Zincite syntax or reopen base-044 through base-047.

## Done when

- [x] Shared callable map ranking and the common sort_by(value) map-selector form accept the existing homogeneous scalar categories and reject nested or incompatible values. Preserve exact integer/float comparison, null/Boolean behavior and lexical-key tie breaking without changing constructed sort admission.
- [x] An explicitly ordered result retains ranking through bindings and reverse/head/tail. Reversal reverses the entire order, including ties; unranked maps remain lexical. Zirium textual JSON and Markdown retain their existing output order and shape.
- [x] Zirium delegates ranking and ordered-entry selection to Zirql. Comparisons, key/value access, temporary ordering storage and result copies share the active budget; rendering stays host-owned and no JSON serialization is required by the core algorithm.

## Validation

- Reuse ranked-map tests for tally/map_by inputs, ties, reversal/truncation and round-tripping through bindings. Include one precision-sensitive integer/float comparison and refusals for nested/mixed scalar categories; retain existing JSON/Markdown checks.
- Run required Zirql and affected Zirium Rust checks, the affected Rust/Python query suites and CLI controls, git diff --check and zdev check base. Reuse existing fixtures and focused public-behavior tests; no new benchmark framework or broad test matrix.

## Result

Share exact scalar map ranking and ordered entry selection; Zirium preserves ranked order through bindings, bounds, JSON and Markdown.

Validation:

- Independent whole review 1 passed with no corrections; snapshot Wf4719b29dc2fc58f and both repository index/byte guards unchanged.
- Stable and Rust 1.88 core formatting, strict Clippy, 120 tests and 8 doctests passed; Zirium 647 tests per toolchain, 3 doctests and warning-denied rustdoc passed.
- Private CPython 3.11–3.14 each passed 27 query/diff tests, Ruff and ty against the real Git dependency; canonical artifacts unchanged.
- Core and consumer production sites, desktop/mobile UI and link checks passed; git diff and zdev checks passed.
