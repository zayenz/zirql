+++
schema_version = 1
id = "base-020"
key = "typed-query-inputs"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-019"]
+++
# Supply typed query parameters through both host CLIs and libraries

## Outcome

The first two example programs run unchanged with typed caller inputs through inline, file and library entry points.

## Context

Read the brief's exact param and --param contract and examples 1/2. Start at src/program.rs Bindings::new, the shared frontend introduced by correlated projection, and both consumer parse/evaluate/CLI boundaries. Primitive predicate arguments must accept checked expression values rather than source substitution.

## Boundaries

- Zirql owns declaration checking, scope, admission and argument validation; hosts transport values and implement domain operations.
- Initial declarations support int, string and bool only. No defaults, imports, functions, named result schemas, undeclared implicit parameters or query-text interpolation.

## Done when

- [x] param declarations and typed library inputs share the existing immutable binding path; duplicate, missing, extra and wrong-type arguments fail before domain execution.
- [x] Zirium and both Zincite query commands implement --param name=value decoding exactly as specified in the brief, in inline and file modes.
- [x] Parameters compose with pure expressions and typed domain predicate/stage arguments, including op(operation_name), under the shared budget and located validation.
- [x] Examples 1/2 produce their specified results for the supplied inputs. Literal-only short commands still need no parameter declarations; parameterized inline/file/library behavior agrees.

## Validation

- Use the two existing example controls for successful binding and one quoted string treated strictly as data; focused argument refusals cover declaration and CLI decoding boundaries.
- Run required checks in all affected repositories and their focused CLI/library tests; preserve current emission/error behavior.

## Result

Shared typed int/string/bool admission now powers Zirium text, Rust builders and Python, plus Zincite Program libraries and both CLIs. Ordered declarations, exact raw values, native results and one budget agree across inline/file/library examples (5/2 and 18/19). Core API cbf7fc84b12c779916144ae004d639c466e506eb; Zirium cd8d90e19eeac5a0ac2f82630c2d55795549cd0f; Zincite 54f8990dc909fc0dc6b46c46a7c6be6c95a27f03. Consumer commits are pushed separately.

Validation:

- Independent whole-task review and fresh focused recheck PASS at W9528e39bd3f4ad36; all four done conditions, exact source/contract/pin states and final completion comparison accepted. The one UTF-8 comment-scanning defect is corrected with a focused native control.
- Core fmt, warning-free workspace/all-target Clippy, workspace tests and doctests, diff and zdev checks pass. Both immutable Git dependencies resolve the reviewed API.
- Zirium Rust 1.88/stable full workspace/all-target/all-feature serial tests and Clippy, fmt, doctests and warning-free API docs pass. Rebuilt native CPython 3.11–3.14 each pass Ruff format/lint, ty and full pytest: 166 passed, 2 unchanged macOS /dev/full skips.
- Zincite fmt, warning-free workspace/all-target Clippy, workspace tests and diff pass, including independent reruns after the Unicode scanner fix; both CLI commands accept Unicode trivia and preserve outer diagnostics.
- Public native Rust/builder/Python and six inline/file CLI report forms agree; refusals occur before native work/output, exact quoted strings remain data, written-position scope and one-budget behavior are preserved. Parameterized data replacement retains original/comment/CRLF bytes, validates the complete candidate, and rejects bad admission without stdout or writes; MiniZinc instance checks pass.
- All three site check/build groups pass; five affected rendered guides, canonicals and 249 local links/anchors checked. Guides updated without site publication or prerelease.
- All 12 Zirium CI jobs pass for cd8d90e19eeac5a0ac2f82630c2d55795549cd0f: https://github.com/zayenz/zirium/actions/runs/38447857053.
