+++
schema_version = 1
id = "base-137"
key = "optional-native-sequences"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Admit present native values in optional sequences

## Outcome

sequence<opt Native> accepts compatible present native sequence elements in both hosts.

## Context

Both hosts reject `fn size(d:sequence<opt Node>)->int {1}` or the Operation equivalent when passed a constructed sequence of present native leaves, while scalar optional sequences work. Exact declaration equality appears in both hosts' function admission and worklist extraction. Reuse Zirql runtime_types compatible_declarations_with and actual per-member admission.

## Boundaries

- Keep existing declaration compatibility; do not introduce a new covariance system or change scalar/effect policies.

## Done when

- [x] A present Node/Operation sequence passes the corresponding optional-element parameter in ordinary functions and worklist extraction.
- [x] Incompatible empty schemas and wrong-owner or wrong-side values are still refused.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Add or extend one focused public optional native-sequence control per host; reuse incompatible-empty and ownership controls.

## Result

Present native sequences satisfy compatible optional-element parameters and worklist inputs. Each member still passes host admission, and empty sequences retain their element declarations.

Validation:

- Required Rust checks passed in Zirql, Zincite and Zirium; focused Zirium correction recheck passed with native-owner and empty-schema controls.
- Astra medium reviewed added prose. Both consumers use canonical Zirql 0074bc583fee19603c796edf541ac443f858e7ed.
- Independent verifier passed; exact core snapshot and consumer states matched before completion.
