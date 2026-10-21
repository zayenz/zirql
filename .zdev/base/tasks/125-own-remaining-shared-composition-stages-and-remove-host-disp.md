+++
schema_version = 1
id = "base-125"
key = "core-composition-stages"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-124"]
+++
# Own remaining shared composition stages and remove host dispatch

## Outcome

Common filter, match, ordering, mapping, existence, fixed-point and path composition execute through core stages while hosts supply domain leaves and policies.

## Context

Read brief.md sections Consolidate language machinery and data rendering (2026-10-21) and Complete the common language boundary (2026-10-21), including their compatibility, documentation and validation contracts. Continue the representation adopted by core-value-projection-stages. Inspect src/reader.rs filter/matching/ordering/mapping/where_exists/fixed_point/paths hooks and their existing shared implementations. Zincite program/composition.rs scoped_value_arguments/evaluate_stage_with_arguments and Zirium query/report.rs retain repeated shared-body discovery and forwarding. Zincite gives existence and fixed-point bodies different relative-input contexts; preserve the established semantics.

## Boundaries

- This migrates existing common composition, not domain navigation algorithms, new operator overloads or graph storage.
- Preserve host-specific native empty-stream policies, incomplete/Unknown facts, effect restrictions and source/side ownership. Keep native stage argument hooks only for native stages.

## Done when

- [ ] The common forms listed in the outcome use core-owned preparation, scoping and execution, including their already-supported ordering/grouping/deduplication variants; domain leaves remain host callbacks.
- [ ] Both consumers delete the replaced common stage variants, reader constructors, nested-argument discovery, purity traversal and forwarding branches. Necessary native kind/admission checks remain explicit.
- [ ] Match capture scope, predicate subjects, selector subjects, existence/fixed-point input, stable ordering and bounded path behavior agree with existing public programs, including empty and unknown inputs.
- [ ] Embedding and both host architecture docs show the remaining domain hooks; affected library-reference entries and sidebar links describe actual supported composition.

## Validation

- Reuse existing filter/match, mapping/ordering, existence/fixed-point and path controls; add only missing public behavior at a changed boundary, especially relative inputs and a later failure. Avoid a cross-product of forms and hosts.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.
