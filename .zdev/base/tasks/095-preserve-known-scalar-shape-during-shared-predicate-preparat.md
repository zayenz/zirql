+++
schema_version = 1
id = "base-095"
key = "scalar-predicate-shape"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-094"]
+++
# Preserve known scalar shape during shared predicate preparation

## Outcome

Shared predicate preparation rejects structurally impossible required arguments and scalar field access before enumeration, including when a scalar source is empty and its primitive kind is unknown.

## Context

During base-087 correction1, filter_elements([a] => true) on a=[] and head(0) | filter_elements(v => v.foo > 0) passed preparation. The adapter knows the subject is a present primitive scalar but has no element from which to obtain Boolean/Integer/Float/String facts. Independently assessed existing APIs cannot express that fact; even actual Integer/String facts do not currently reject required list headers. See value.rs Analysis::scoped_types, value/pattern.rs facts, runtime_types.rs and existing ValueType shape facts. Reuse the existing type/pattern machinery and cumulative Budget. Read the brief Scoped predicate expressions, Shared destructuring patterns and Testing sections. The bounded assessment /private/tmp/zirql087-scalar-assessment.json and public reproducer /private/tmp/zirql087-scalar-fact-gap.rs explain the concrete missing seam; source remains authority.

## Boundaries

- Add only the metadata needed to express known primitive scalar shape with unknown primitive kind, analogous to existing shape-only facts. It is not a runtime value or new declared/surface type. Actual primitive facts remain more precise; preserve scalar shape through ordinary fact propagation instead of globally erasing it to Unestablished.
- Reuse existing pattern recursion for known-shape admission of required predicate arguments before body preparation. Preserve ValueMatch arm mismatch/fallback semantics, source locations, native identity and the same finite Budget. Compatible primitive operations may defer exact primitive kind; established impossible structural access must fail. No inference/exhaustiveness framework, union types, alternative evaluator, host matcher, fake primitive, identity probe or retry-until-success preparation.
- Own core API/implementation, focused nearby behavior coverage and concise adapter documentation. No consumer source/pin edits, registry publication, tags/assets or deployment. Root owns records, transport, pins and serial acceptance. This prerequisite preserves base-087 correction1 and its one remaining focused review; it does not reopen completed core tasks or reset consumer review allowances.

## Done when

- [x] A host can provide known primitive scalar shape without inventing an actual primitive kind. Required list/tuple/record predicate headers and scalar field/index access reject that fact before enumeration; actual primitive facts also reject impossible required structural headers.
- [x] Valid scalar numeric predicate bodies remain preparable on empty unknown-kind sources, while incompatible established primitive kinds still fail. Existing runtime matching remains responsible for values not established by preparation; scalar metadata never becomes a runtime value.
- [x] ValueMatch may still try a structural arm against a scalar and fall through to a later wildcard. Existing valid destructuring/native scopes, locations and cumulative limits remain coherent. A few nearby public behavior controls cover these boundaries without a syntax/position matrix.
- [x] Required Rust/MSRV/docs checks pass; adapter documentation states the metadata and required-pattern boundary. Read-only inspection of the Zincite087 seams confirms sufficiency, and both consumers plus CSV compile against the source API without broad adapter changes. Consumer final pins and release artifacts remain separate tasks.

## Validation

- Run fmt, warning-fatal workspace all-target Clippy, workspace tests, Rust1.88 tests, warning-fatal docs/doctests, git diff --check and Root canonical zdev check.
- Use existing tests for scalar/actual-primitive required header refusal, scalar structural access refusal, valid unknown-kind numeric preparation, incompatible actual String, and ordinary ValueMatch fallback; keep coverage proportional to the exhibited defect.
- Compile the two consumers and CSV against a disposable local copy/path adaptation without claiming final Git or registry resolution; preserve product sources and consumer work. If reader prose changes, run affected site build/link checks and Root proportional desktop/mobile review.

## Result

Added preparation-only scalar shape facts and required predicate structural admission; independent whole review and focused recheck accepted the complete correction.

Validation:

- Stable/MSRV workspace tests, fatal Clippy/docs/doctests, CSV and both disposable consumer library builds passed.
- Compatible membership/dictionary/keyed operands pass; actual-kind, key-rank, nominal and scalar structural refusals and ValueMatch fallback are preserved.
- Site checks and 20 guide links passed; exact desktop/mobile guide hashes verified by Root. Final Git pins, packages and registry checks remain consumer/release tasks.
