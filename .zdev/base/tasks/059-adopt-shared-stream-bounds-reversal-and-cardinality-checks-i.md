+++
schema_version = 1
id = "base-059"
key = "zincite-native-stream-controls"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-048"]
+++
# Adopt shared stream bounds, reversal and cardinality checks in Zincite

## Outcome

Zincite native pipelines and saved results use Zirql tail, reverse and pass-through cardinality checks, beyond the existing head/count stages.

## Context

Base-048 supplies shared native stream algorithms and Zirium migration; its Zincite work is the Boolean require checker, not native stage adoption. Zincite language.rs and pipeline.rs currently expose Head/Count/Unique/Tally; program.rs::validate_stages identifies the native stream kinds admitted by head/count. Reuse those bounded streams and their public metadata, not new value categories. Read the alignment brief, base-048 and Zincite native pipeline/program/composition adapters. Wait for integrated Zincite base-140 and coordinate other consumer work before editing.

## Boundaries

- Reuse 048 algorithms and its common spelling/argument rules. Preserve Zincite terminal emitters, transformations and original-input/data policy. Do not duplicate the expression require checker, add constructed-array bounds, or introduce sorting, map/set/fixed-point operations in this slice.
- Limit admission to the existing native stream kinds supported by Zincite head/count. Keep errors and limitations explicit; a cardinality check of returned items does not establish complete domain knowledge.

## Done when

- [ ] The read-only Query and Program APIs and both CLI forms support tail(n), reverse and 048 check forms on the existing node/string/literal/semantic stream kinds, including a saved binding followed by these stages. Stage arguments follow existing scoped-argument admission where applicable.
- [ ] Tail preserves the retained suffix order; reverse reverses actual input order including duplicates. Successful checks pass the original stream through; empty/exact-count failures and optional messages follow 048. Unsupported value kinds still fail before domain execution when known.
- [ ] Zincite delegates selection/cardinality algorithms to Zirql while retaining native identities, original ranges, semantic owner and completion/limitation metadata. Nested work, copied/retained results and output buffering use the same request budget. Document the new native stages without claiming broader language alignment.

## Validation

- Reuse existing head/count/native program controls for zero/oversized tail, reversal with duplicates, pass-through and failed checks, saved-result chaining and a semantic result retaining limitations. Exercise one scoped bound and one unsupported-kind refusal; reuse the existing CLI no-partial-output control.
- Run required Zirql checks if shared code changes, Zincite workspace formatting/strict Clippy/tests and affected CLI controls, git diff --check and zdev check base --format json. Record separate consumer commits; no benchmark or broad test matrix.
