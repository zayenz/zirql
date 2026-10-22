+++
schema_version = 1
id = "base-138"
key = "zincite-dictionaries"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-136"]
+++
# Use retained dictionaries throughout Zincite reports

## Outcome

Zincite map_by results and dictionary declarations containing native values use the same retained dictionary behavior as constructed dictionaries.

## Context

With input `int:a=1; int:b=2;`, `let d=items | map_by(names,count); length(d)` and d["a"] fail, while d | count and equivalent constructed dictionaries work. composition.rs mapping_kind/construct_map retains the old Record representation, but dictionary_view sees only Dictionary. functions.rs declaration_kind also rejects all native-bearing dictionary declarations, despite existing GeneralDictionary admission. Use the shared admission path from dictionary-admission.

## Boundaries

- Keep model lookup and source ownership in Zincite; do not stringify keys or add compatibility wrappers for obsolete storage.

## Done when

- [x] map_by results support length, lookup and the existing common dictionary operations with their actual entries.
- [x] Functions admit and return dict<Node,int> and dictionaries containing native sequences, including typed empty results.
- [x] Insertion order, retained owners, declared schemas and documented output shape are preserved; replaced map dispatch and storage are removed where no caller needs them.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Extend existing public dictionary/function controls for map_by length/lookup and native-bearing function declarations; include an empty declared result.

## Result

Zincite map_by results support common dictionary operations. Functions accept and return dictionaries containing native values, and empty results retain their declared schemas.

Validation:

- Required offline locked Rust formatting, Clippy and tests passed in affected repositories; Zirium doctests and warnings-denied docs passed. Focused public controls passed.
- Astra medium reviewed added prose using unslop and zayenz-writing-style. Both consumers use canonical Zirql e1b36d26adf2a509701ec85688aa0319ea4dc671.
- Independent verifier passed; exact core snapshot and consumer states matched before completion.
