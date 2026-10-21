+++
schema_version = 1
id = "base-133"
key = "coherent-infix-comparisons"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-125"]
+++
# Align infix comparison admission for native and constructed data

## Outcome

Equivalent ordinary data has the same infix comparison and equality behavior whether constructed directly or retained through a native collection view, while genuinely native leaves retain domain identity.

## Context

Read the brief's Coherent native and constructed operations decision and the existing comparison/equality language contract. A temporary public FunctionRequest probe using the existing tests/native_programs.rs SourceHost fixture showed native list [1] < [2] succeeds while the constructed equivalent fails numeric admission; native [1] == [true] returns false while the constructed equivalent fails compatible-type admission. Tuples exhibit the same issue. src/functions.rs compute_collection dispatches native operands directly to native_values_compare/native_values_equal after unavailable facts became Unestablished, bypassing src/value.rs binary_type/binary/compatible_types/equal_values_impl. This is established callable-library behavior, not a claim of consumer CLI reproduction. Keep ordinary operator semantics and apply them consistently rather than widening them to match structural sorting.

## Boundaries

- The fixed operation list is <, <=, >, >=, ==, = and !=. Keep the established ordinary numeric relational and compatible-type equality contracts. Do not add structural infix ordering, scalar coercion, new operators, a second evaluator or a generic normalization framework.
- Preserve host admission and exact identity for genuinely native leaves, including owner/source/diff side. Do not compare rendered text or erase declared optional, nominal or keyed-shape contracts. Native-only data without a constructed counterpart remains subject to its explicit domain contract.
- Use existing preparation, deferred facts, native views and value helpers. Known misuse retains preflight refusal; deferred checks occur when evaluated and preserve lazy branches. Do not infer complete schemas by scanning every input row.

## Done when

- [ ] Existing relational operators accept present signed integers and finite floats with the same exactness rules in both representations; mixed numeric comparisons still require exactly representable integer conversion. Native list/tuple/other ordinary container wrappers do not bypass the ordinary refusal for nonnumeric operands.
- [ ] Existing equality aliases and inequality enforce the same compatible underlying-type admission for equivalent ordinary data in either representation. Preserve positional array/tuple equality, set membership, dictionary associations, record field compatibility, declared optional absence and nominal/keyed-shape rules.
- [ ] Unavailable native facts defer validation rather than waive it. Runtime checks use the active budget and located errors, preserve genuinely native identity, and do not evaluate an untaken lazy branch merely to discover its values.
- [ ] Text and callable/typed paths that expose these existing forms agree; both consumer adapters use the corrected common behavior where applicable, with no per-frontend or per-host legacy-policy flag.
- [ ] Update the comparison/equality reference and Rust embedding/API documentation, plus affected host support notes. Explain that numeric infix relations and structural sorting have different operator contracts, while neither changes merely because ordinary data is carried natively.

## Validation

- Reuse existing numeric/equality, native identity and lazy-branch controls. Add a compact public regression using the existing native SourceHost fixture for one ordinary compound relational refusal and incompatible equality refusal, alongside an accepted equivalent comparison. No new fixture framework or representation/container matrix.
- Run the brief's required Rust checks and affected host/Python checks only in changed repositories. Run affected docs/example/link checks, git diff --check and coordinator zdev check base --format json; retain separate consumer commits and accepted Zirql pins if integrations change.
