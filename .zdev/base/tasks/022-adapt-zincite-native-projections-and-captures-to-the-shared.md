+++
schema_version = 1
id = "base-022"
key = "zincite-native-composition"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-021"]
+++
# Adapt Zincite native projections and captures to the shared language

## Outcome

Zincite uses the same projection and capture evaluator for declaration/use reports, demonstrating domain adaptation rather than a second language implementation.

## Context

Read the brief's shared ownership direction, examples 1/3 for generic semantics, and Zincite semantic-query docs and public ModelFacts APIs. Its declarations/uses stages already retain proven identities, locations and completeness; use them as domain primitives.

## Boundaries

- Reuse shared parsing for new generic constructs, adapting embedded domain query syntax and MiniZinc value expressions where necessary. Keep one shared validator/evaluator for capture and projection semantics.
- Model loading, reference resolution, identity and report limitations stay in Zincite. No new semantic proof engine or changes to the attended data-edit contract.

## Done when

- [x] Zincite text and library programs project each native declaration into a correlated record of its name and use count without requiring unique names.
- [x] A shared match with dependent declaration/use generators retains both native subjects and can project their names/text; scope/order/purity are the same as Zirium's core path.
- [x] Known matches from incomplete input retain Zincite completeness and limitations; uncertain required guards refuse with the responsible host location rather than becoming false.
- [x] Both Zincite commands demonstrate a compact --program query and its file equivalent. A self-contained small model avoids solver/external-service requirements and proves no host-local matcher or projection loop is needed.

## Validation

- Reuse a focused declaration/use fixture with an identical-spelling/different-identity control and one existing incomplete-facts control; compare native results before rendering.
- Run required Zirql/Zincite checks and focused CLI/library tests; retain attribution of consumer changes separately.

## Result

Zincite now adapts native declaration projections and dependent declaration/use captures to the shared Zirql reader, validator and evaluator. Native handles, correlated rows, model ownership, completeness and located Unknown refusals survive both command forms. Consumer integration commit 1795de06e2c8f384d2e265becd0732caf4efa333 uses reviewed Zirql Git commit d7f25e642f59ce09920b81ea169ce3bb2eee3cbd; both are pushed. No core API or data-edit contract change.

Validation:

- Independent initial whole-task PASS at W2fc59ae6ceaf1df9; all four conditions and 17 attributed consumer paths checked, with no corrections.
- Native library results preserve distinct same-name declaration IDs and counts 2/0/0/1, ordered declaration/use capture IDs and exact 138-byte BOM/CRLF source bytes before rendering; empty schemas, scope, purity, cardinality, continuing limits, recursive native record copying and captured-item fragment rendering pass.
- Actual standalone and umbrella inline/file controls agree: complete reports exit 0, incomplete known matches exit 1, required Unknown at its model location and later errors exit 2 with empty stdout; original model bytes and report-only edit refusal remain intact.
- All 12 independent required checks pass: core and Zincite fmt, Clippy, workspace tests including doctests and diff checks, core zdev check, Zincite site check and both exact-base site builds. Actual dependency metadata resolves the reviewed Git source at alpha.5.
- Root independently checked landing, changed query guide, reference examples/tables, links and real search results, keyboard focus/dismissal, themes, assets/canonicals and mobile navigation/overflow at both base paths. Source-bound UI evidence remains unchanged; no deployment.
- Fresh completion comparison and all three repository states, original unrelated core files and committed consumer source bytes match the verified state. Staged whitespace validation preserves the intentionally relocated CRLF fixture with cr-at-eol enabled only for that check.
