+++
schema_version = 1
id = "base-089"
key = "zirium-shared-predicates-integration"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-091", "base-083"]
+++
# Integrate shared predicates into Zirium and its diff language

## Outcome

Zirium uses the reviewed shared named and unnamed predicate language in its normal and diff queries, with updated core contracts and reader examples.

## Context

Consume the actual independently reviewed Zirql 0.1.0 scoped-predicate and shared-pattern API from base-086/base-090/base-091 and the base-083 packaging contracts. Read the brief's Shared destructuring patterns and One shared query reader sections. Adapt existing normal/diff parser hooks and exhaustive pattern/generator walkers to that contract, retaining the actual structural views each domain supports. Read the brief's complete predicate inventory and subject/scope rules. Normal query/parser.rs and comprehension.rs split native and constructed filter routing; report.rs/program.rs/functions.rs walkers retain prepared bodies and active calls. query/capture.rs owns native match/path callbacks; query/pipeline.rs and query.rs root traversal evaluate each candidate operation/ancestor. Separate query/diff_text.rs and diff_program.rs parse/evaluate diff filter/root with existing change and side-owned operation meanings. They need a small shared value bridge over existing native queries/projections, not a separate predicate evaluator or synthetic record API. The read-only /private/tmp/zirql-predicate-zirium-inventory.json identifies current seams; inspect actual source. Zincite integrates independently; final acceptance checks both consumers and CSV.

## Boundaries

- Keep existing MLIR/diff subject meaning, native/SSA ownership and identity, before/after sides, source coordinates, domain leaves, edits/effects and complete-output buffering. Named/unnamed syntax must work across the complete existing predicate inventory without inventing native fields, new diff declarations or host-language callable closures.
- Use Zirql's shared query reader for generic expressions, function and predicate bodies, ordinary expression-valued native arguments and constructed comprehensions. Retain native MLIR/diff parsing and narrow domain atoms only where domain facts are required.
- Use shared Zirql parsing/preparation/execution and the active prepared function runner with cumulative budgets; no fresh request/session per element, independent evaluator, optional language extension or new platform/compiler/Python matrix.
- Root owns Git transport/pins/locks/zdev and serial acceptance. No registry publication, release tags/assets or site deployment.

## Done when

- [ ] Common for/in/if and MiniZinc pipe/where comprehensions use the shared reader in existing CLI/library, native and function contexts. Patterns, sequential scope, ordering, types and cumulative limits agree; generic query parsing has no broad host-owned fallback. Lead guides with the common form and explain the alternate spelling. Preserve actual domain parsing and admission policy.
- [ ] Shared exact tuple/list and discarded-prefix list patterns work consistently in existing let bindings, function/predicate arguments, comprehension binders and value-match arms. Pattern mismatch errors and match fallback agree with core; predicate _ still reads the whole subject. Native/diff subjects without an established structural view refuse rather than inventing fields or converting representations.
- [ ] Constructed/native filter, root, paths targets, native match guards and diff filter/root admit named and implicit _ predicate expressions through shared rules. Every context retains its actual subject: singleton operations, each root ancestor, candidate path target, original match input plus named captures, or side-owned diff change/operation. Existing native/diff leaves compose with general supported Boolean expressions.
- [ ] Prepared calls/captures and typed native values survive saved/nested/function contexts where supported. Nested _ scope and explicit outer captures agree with core, empty/type/purity failures preflight with locations and finite budgets, and wildcard patterns retain their meaning. Diff keeps its existing expression/domain support without new declaration machinery.
- [ ] A few existing-style public library/CLI controls show constructed filter-comprehension equivalence/order, named/unnamed capture/function behavior, representative root/path/match/diff boundaries, typed-empty refusal and later-error empty stdout. Existing Rust/Python builder contracts and native identities remain usable; no form-by-position matrix.
- [ ] Rust/Python versions remain0.5.0 and dependency manifests/locks pin actual reviewed core with registry0.1.0 contracts. Reader/reference/release notes and runnable examples match the new predicates and current support limits. Required Rust/MSRV/Python/docs/site/link/desktop/mobile checks pass; old pre-predicate artifacts are superseded for final acceptance.

## Validation

- Run required stable fmt/fatal all-target/all-feature Clippy/tests, existing Rust1.88 checks, doctests/fatal docs. Explicitly rebuild current Python extension and verify import/distribution identity before the existing pytest/Ruff/ty suite. No matrix expansion.
- Replay a compact shared-pattern program and a located short-list refusal with empty stdout across public query/CLI paths, without a form-by-position matrix. Include a paired common/alternate comprehension through meaningful existing contexts. Use focused public query/CLI/native/diff controls for distinct execution boundaries, prepared scopes, typed empties and buffering. Inspect all normal/diff parser routes and exhaustive scope/purity/free-name/prepared-argument walkers.
- Run affected site check/build/local links and Root desktop/mobile/search review. Inspect pins/normalized version contracts; final clean artifacts/hosted jobs/all-host acceptance remain base-084. No public registry or deployment claims from local results.
