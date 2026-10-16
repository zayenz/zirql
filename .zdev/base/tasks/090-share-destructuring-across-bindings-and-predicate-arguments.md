+++
schema_version = 1
id = "base-090"
key = "consistent-destructuring-patterns"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = ["base-086"]
+++
# Share destructuring across bindings and predicate arguments

## Outcome

One shared pattern language handles tuple and positional-list destructuring consistently in bindings, parameters, predicate arguments, comprehensions and value matches, including the CSV example.

## Context

Read the Shared destructuring patterns section in the brief, which records the user's exact mismatch choice and supersedes the earlier exclusion of discarded list rests. Consume base-086's actual reviewed scoped-predicate API. Begin with src/value/pattern.rs (PatternKind, facts, bind, copy/disposal), reader.rs pattern and comprehension binder readers, value/prepared.rs and execution.rs, functions.rs pattern/native declarations, and examples/csv.rs read_program. Existing tuple/record patterns already serve lets, typed parameters and value-match arms; predicates currently store one optional name and comprehension generators store simple binders. The CSV prefix reader currently only admits named lets. Reuse existing pattern machinery rather than adding another binder or evaluator. Host integrations follow this core task and must adapt their exhaustive walkers and native views.

## Boundaries

- Keep one pattern contract and existing host/native ownership, shape, nominal identity, lexical scope, source ranges and cumulative work/result/nesting limits. Preserve grouped-generator evaluation/order and all-branch/typed-empty admission.
- Support only exact positional-list patterns and a final discarded .. prefix remainder. Do not add named tails, arbitrary extraction calls, first-class functions, reflection, implicit keyed/set/tuple conversions or new native selection capabilities.
- Core/CSV code, focused public coverage and current-language documentation only. Do not edit consumer repositories, publish packages, push release tags or deploy sites. Root owns pins, transport, lifecycle and final acceptance.

## Done when

- [ ] Existing names/wildcards/tuples/records/literals/presence and new exact/prefix list patterns use shared parsing, facts and matching in let bindings, typed function parameters, predicate arguments, comprehension binders and value-match arms. Existing named and implicit predicate forms remain usable.
- [ ] Subjects evaluate once, successful pattern names appear atomically, and mismatch fails with a located error except that value-match tries the next arm. Predicate _ denotes the whole subject even with destructured parameters, nested _ rebinds, named outer captures remain lexical, and duplicate/shadow/free-name/type/purity and typed-empty checks remain deliberate.
- [ ] List prefixes retain actual positional/native child values and contracts without copying or binding the ignored tail. Tuple/list distinction, host owners, indexed shape and finite budgets survive. Ordinary hosts and the active prepared function runner can both use the same scoped patterns.
- [ ] A few public controls demonstrate tuple and list-prefix reuse across the binding positions, a pure predicate call and nested whole-subject _, short-list/duplicate refusal, match fallback and buffered failure. Packaged CSV runs a destructured let and row predicate using the shared language without adding named function declarations.
- [ ] Required stable/MSRV Rust, docs, package example and site/link checks pass. API/language/host guides and release notes describe current forms; Root independently verifies and commits the actual source for both dependent integrations.

## Validation

- Run required fmt, fatal workspace/all-target Clippy, workspace tests, Rust 1.88 tests and warning-fatal docs/doctests. Extend only focused existing public pattern/predicate controls; avoid a form-by-position matrix.
- Inspect every pattern and generator scope/copy/disposal/prepared/native walker, including retained aliases and typed empty preflight; use temporary checks where a permanent test adds no distinct confidence.
- Exercise extracted packaged CSV sources for a destructured let and predicate, named/implicit input and a located mismatch with empty stdout. Run affected site check/build/links and proportional Root desktop/mobile review. No publication.
