+++
schema_version = 1
id = "base-094"
key = "scoped-native-argument-kinds"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "high"
blocked_by = ["base-092"]
+++
# Preserve native operand kinds through shared predicate scopes

## Outcome

Named and implicit predicate scopes preserve native argument result kinds through shared preparation, so native leaf operands can reuse the shared matcher and active runner.

## Context

The two active consumer integrations independently found scoped native operand gaps. Zirium same_value(operand(original | filter(op("example.right")),0),operand(right,0)) under a named match-where header cannot retain the header/native result kind. Zincite name(n,"a") under n=> needs the same scope and pre-enumeration native kind admission. PredicateArgument::bind_value already builds SubjectScope and the active runner executes it, but functions.rs Preflight::local_kind has no SubjectScope arm, falling back to constructed/unknown facts. Root independently inspected this omission and the retained NativeValue kinds. Preserve the selected body kind under the existing shared pattern/whole-subject scope, using current pattern metadata and scope restoration rather than host matchers/private subject names. Zincite also needs signature-only kind discovery from its existing PatternFunctionDeclarationRef headers inside a native kind callback: pattern_function_value_kind accepts owned declaration containers, while PreparedFunctions already supports borrowed headers. Add the narrow borrowed-header counterpart using the same signature registration and entry-kind preflight, without preparing declaration bodies or cloning them. Read the brief Scoped predicate expressions/Shared destructuring patterns/Testing, Preflight::local_kind, prepare_predicate, pattern_kind, entry_kind, and existing value-kind/borrowed preparation APIs.

## Boundaries

- Use existing SubjectScope, pattern matcher, lexical metadata, native kinds and request-local runner. No new parser/evaluator, synthetic predicate or identity-probe stage, per-element function request, owned body/header scaffolding, duplicated matching, reflected fields or serialization.
- Kind discovery checks signatures and the selected expression; it does not establish purity of declaration bodies or authorize execution. Preserve complete body preparation as a separate required request step, native owner/runtime admission and one cumulative Budget.
- Own functions.rs/public export, focused existing native fixture coverage and concise adapter/API guidance. No consumer source/pin edits, registry publication, release tags/assets or deployment. Root owns records, transport, pins and serial acceptance. Reader task093 runs in a different source and owns field continuation; coordinate shared test/doc edits to remain disjoint.

## Done when

- [x] SubjectScope kind discovery retains an actually native body result under named, destructured or implicit subject scope, with correct nearest whole-subject metadata and outer captures. Nested scopes restore the outer facts on success/error; unknown constructed facts do not inherit an outer native subject kind.
- [x] A public borrowed-pattern-header value-kind helper accepts existing PatternFunctionDeclarationRef values and shares the existing signature-only registration/entry-kind behavior. It supports use from a native kind callback without preparing all bodies, copying declaration trees or creating per-row requests; the owned-container helper remains coherent.
- [x] A focused public native argument fixture obtains the same native kind and actual native result before and after bind_value/SubjectScope, covering named or implicit access and one empty/located refusal boundary. Borrowed and owned headers agree on kind discovery, including a callback use, while complete preparation still rejects an invalid unused declaration body. Reuse nearby controls; no position/syntax matrix.
- [x] Required stable/MSRV/fatal Rust/docs checks, git diff and zdev checks pass. Current adapter docs explain the scoped native kind and borrowed-header discovery boundary. Root independently accepts the exact revision before both hosts finalize their argument wrappers and actual pins.

## Validation

- Run required fmt/fatal workspace all-target Clippy/workspace tests/Rust1.88 tests/warning-fatal docs and doctests, git diff --check and Root canonical zdev check. Preserve existing native owner/SSA identity, pattern mismatch, nearest-subject restoration and native argument runner controls.
- Use a few existing-fixture assertions for native kind preservation and execution through SubjectScope and borrowed-header kind discovery. Check typed-empty or skipped invalid operands before enumeration and located failure under the current allowance. Do not add another matcher, probe stage, body-preparation loop or coverage matrix.
- Inspect the concrete Zincite guard kind callback and Zirium SSA operand preparation read-only to confirm the public boundary suffices. Their actual implementation/runtime checks remain087/089. If reader-facing prose changes, run affected site check/build/local links and Root proportional desktop/mobile review. Final release artifact/registry/installation acceptance remains084.

## Result

Preserve native operand result kinds through shared predicate scopes and expose borrowed-header signature-only kind discovery.

Validation:

- Independent whole review passed; stable fmt/fatal Clippy/workspace and Rust1.88 tests/fatal docs and doctests/diff/zdev checks pass.
- Native runner, nested subject, callback and located refusal controls pass; all20sitepages/1015links and exact merged desktop/mobile guide proof pass.
