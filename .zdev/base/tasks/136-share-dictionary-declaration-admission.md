+++
schema_version = 1
id = "base-136"
key = "dictionary-admission"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Share dictionary declaration admission

## Outcome

Both hosts use Zirql for common dictionary schema and entry admission.

## Context

Nearly identical schema admission rules appear in Zincite program/functions.rs admit_dictionary_schema, Zirium query/functions.rs and query/diff_program.rs. These paths recursively strip optional, inspect nested dictionary schemas and string-key streams, and leave native facts to the host. Per-entry admission is also repeated. Start with src/runtime_types.rs and the existing declaration and dictionary views.

## Boundaries

- Use existing declarations, views and narrow host callbacks. Do not add a generic type engine or a new crate.
- The hosts retain native model facts, owner validation and diff-side policy.

## Done when

- [x] One focused core admission path replaces the duplicated dictionary schema and entry rules in ordinary Zincite, ordinary Zirium and Zirium diff.
- [x] Empty declared dictionaries, nested dictionaries and native keys/values retain their admission, owner and side checks.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Reuse existing declaration/dictionary tests, adding only a missing public control for a changed contract.
- Exercise a dictionary with native contents and an incompatible empty schema in each affected admission path.

## Result

Zincite, Zirium and Zirium diff use shared dictionary admission. Retained schemas are checked before entries, including empty dictionaries; native ownership and side checks remain with the hosts.

Validation:

- Independent required Rust checks passed in all three repositories. Native dictionary, empty schema and ownership controls passed.
- Executable documentation examples passed. Added prose received Astra medium copyediting.
- Both consumers use Zirql b5bee66f912be25264beab10157d19affe2420a2. Consumer commits: {"zincite": "4b492c4f131300b2bb681a0d55f2d55673fcc18a", "zirium": "775759839c5801d884d2484a49b56d7e64b0b87c"}
