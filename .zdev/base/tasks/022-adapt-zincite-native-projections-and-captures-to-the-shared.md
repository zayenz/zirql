+++
schema_version = 1
id = "base-022"
key = "zincite-native-composition"
area = "base"
status = "open"
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

- [ ] Zincite text and library programs project each native declaration into a correlated record of its name and use count without requiring unique names.
- [ ] A shared match with dependent declaration/use generators retains both native subjects and can project their names/text; scope/order/purity are the same as Zirium's core path.
- [ ] Known matches from incomplete input retain Zincite completeness and limitations; uncertain required guards refuse with the responsible host location rather than becoming false.
- [ ] Both Zincite commands demonstrate a compact --program query and its file equivalent. A self-contained small model avoids solver/external-service requirements and proves no host-local matcher or projection loop is needed.

## Validation

- Reuse a focused declaration/use fixture with an identical-spelling/different-identity control and one existing incomplete-facts control; compare native results before rendering.
- Run required Zirql/Zincite checks and focused CLI/library tests; retain attribution of consumer changes separately.
