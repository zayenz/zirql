+++
schema_version = 1
id = "base-127"
key = "common-program-reader"
area = "base"
status = "open"
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

- [ ] Shared code collects function declarations, reads common parameters and pattern lets, introduces names at the proper point and handles separators/final-value termination into located common statements.
- [ ] Both native hosts delegate common assembly and remove its repeated orchestration; any host-only loop or callback handles a named syntax/effect policy rather than duplicating common scope rules.
- [ ] CSV uses the reader for its current let-plus-expression surface and removes bespoke block assembly, let_names maintenance and error cleanup. A plain expression still requires no preamble.
- [ ] Forward function references, sequential parameter/binding visibility, duplicate-name refusal, existing trailing-semicolon rules and implicit/final-output behavior remain compatible on each surface.
- [ ] Common program grammar and embedding docs explain the reader; CSV and both host guides show supported statements, deliberate syntax differences and actual limitations.

## Validation

- Reuse common function/parameter/pattern programs and malformed/trailing-input controls; exercise both host text routes and CSV’s current examples. Keep effect parsing read-only and preserve later-error buffering.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; run documented examples and affected docs content/link checks, git diff --check and zdev check base --format json. Retain separate consumer commits and accepted Zirql dependency revisions.
