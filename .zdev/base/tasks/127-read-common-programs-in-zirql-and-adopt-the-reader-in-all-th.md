+++
schema_version = 1
id = "base-127"
key = "common-program-reader"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-115", "base-124"]
+++
# Read common programs in Zirql and adopt the reader in all three integrations

## Outcome

Zirql owns common declaration collection and statement assembly; Zincite, Zirium and CSV remove their duplicated program parsing.

## Context

Read brief.md sections Consolidate language machinery and data rendering (2026-10-21) and Complete the common language boundary (2026-10-21), including their compatibility, documentation and validation contracts. src/reader.rs already exposes read_statement_prefix, read_pattern_let and function declaration readers. Compare Zincite program.rs Program::parse and program/functions.rs collect_functions, Zirium query/parser.rs program assembly, and examples/csv.rs read_program/let_names. Base-115 supplies expression/name routing; base-117 will consume the resulting lowered program. Preserve existing forward-function and statement-position rules from the brief.

## Boundaries

- Use the existing reader and located program forms with narrow host expression/type/statement hooks, not a configurable parser framework or one mandatory lexer.
- Keep Zincite source/create/replace/append and terminal transaction rules, Zirium legacy binding/do/emitter syntax and each host’s output policy. CSV may retain its current accepted subset until the constructed-program task.

## Done when

- [x] Shared code collects function declarations, reads common parameters and pattern lets, introduces names at the proper point and handles separators/final-value termination into located common statements.
- [x] Both native hosts delegate common assembly and remove its repeated orchestration; any host-only loop or callback handles a named syntax/effect policy rather than duplicating common scope rules.
- [x] CSV uses the reader for its current let-plus-expression surface and removes bespoke block assembly, let_names maintenance and error cleanup. A plain expression still requires no preamble.
- [x] Forward function references, sequential parameter/binding visibility, duplicate-name refusal, existing trailing-semicolon rules and implicit/final-output behavior remain compatible on each surface.
- [x] Common program grammar and embedding docs explain the reader; CSV and both host guides show supported statements, deliberate syntax differences and actual limitations.

## Validation

- Reuse common function/parameter/pattern programs and malformed/trailing-input controls; exercise both host text routes and CSV’s current examples. Keep effect parsing read-only and preserve later-error buffering.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.

## Result

Shared common program reading, protected forward declarations, sequential names, parameters/pattern lets, separators and iterative cleanup across native hosts and CSV. Removed host statement/collection loops and CSV block/name plumbing. Zincite retains native facts, closed Data/edit transactions and output policy; Zirium retains frontend metadata, legacy syntax, native admission and emission. Canonical Zirql dependency 23b4e4e70a9225dfeea51642c130da2431b12d9d; Zincite commit 90467e5c8a6b237b2089696e40ea80213f26b821; Zirium commit b96ce5e0fa7afc2968876379e4e029b4243f7e41.

Validation:

- Independent full review PASS, snapshot Wa911a64bdbb975c5; no corrections and exact core/consumer state comparisons equal.
- Core fmt/Clippy/workspace207; Zincite fmt/Clippy/workspace518; Zirium allfeature/alltarget fmt/Clippy/tests670pass5ignored, doc3 and warning-denied rustdoc passed.
- Rebuilt Python179pass2skip plus Ruff/ty; all3sites check/build/local links6605/732/885 and documented CSV/native recipes passed.
- Shared source-scan/initial-name metadata refusals and existing low-budget native controls passed; git diff checks and zdev check passed.
