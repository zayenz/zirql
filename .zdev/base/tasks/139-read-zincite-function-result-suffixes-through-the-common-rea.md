+++
schema_version = 1
id = "base-139"
key = "constructed-call-suffix"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = []
+++
# Read Zincite function-result suffixes through the common reader

## Outcome

Direct calls returning constructed collections accept the same common suffixes as literal and let-bound collections.

## Context

For `fn rows()->sequence<int>{[3,1,2]}`, direct `rows() | map(x=>x+1)` and windows(2) fail, but `let r=rows(); r | map(x=>x+1)` and literal pipelines work. Zincite functions.rs value_suffix_kind, take_constructed_suffix and recover_call_value maintain partial late-recovery logic. Zirql reader.rs already owns the complete common value suffix grammar.

## Boundaries

- Keep native signatures and MiniZinc source/CST parsing local. Reuse the existing preparation and evaluator.

## Done when

- [x] Direct declared constructed function results support map, flat_map, windows and the other already-supported common suffixes in the same way as their let-bound equivalents.
- [x] Common suffix recognition and lowering use one core grammar; replaced late recovery is removed.
- [x] Native selector and call routing, forward declarations, located errors and effect policy still work.
- [x] Update the affected API or language reference and host guide with current behavior and one useful example. Keep the shared reference and sidebar organized by purpose. All added explanatory prose receives the requested Astra medium copyedit using unslop and zayenz-writing-style.

## Validation

- Run the affected repositories' required Rust checks from AGENTS.md, plus git diff --check. Use the existing offline dependency cache.
- Extend an existing public function program control comparing direct, literal and let-bound suffixes; avoid a stage matrix.
- Exercise the existing small-query preparation benchmark without a new threshold.

## Result

Direct constructed function calls use the same common suffix grammar as literal and saved values. Declared result types are available before function bodies are read, including forward calls.

Validation:

- Independent required Rust checks passed in all three repositories. Native continuations, grouped selectors, forward calls and located errors passed.
- CSV and host guide examples and the existing query_profile smoke passed. Added prose received Astra medium copyediting.
- Both consumers use Zirql bfc799d488365f28b498ff8244f0497461348411. Consumer commits: {"zincite": "59f53f4454d05070f5b0784a5f3e07b8b00937fc", "zirium": "f0802f8ce040e03a0d894db97935673f0d373b21"}
