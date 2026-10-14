+++
schema_version = 1
id = "base-035"
key = "zincite-functions"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-033", "base-031"]
+++
# Expose typed recursive functions in Zincite programs

## Outcome

Zincite users can define typed query functions over native nodes and values, with MiniZinc-shaped expressions and shared tail-call execution.

## Context

Read the common fn/signature contract, Zincite AGENTS.md and its program/CST expression lowering and semantic adapters. Implement query-defined functions, not calls that execute model functions. Translate the brief’s count_calls example to actual syntax-node primitives.

## Boundaries

- Zincite query integration only, plus necessary shared seams; no model execution, solver behavior or broadened data-edit semantics.

## Done when

- [x] Common fn declarations and selected type spellings compose with Zincite’s expression syntax and native-query forms, lowering direct/mutual and tail calls into the shared runtime.
- [x] Host Node/Declaration/nominal contracts retain source coordinates, enum identity and incompleteness; uncertain semantic facts do not become absence or false.
- [x] The syntax-tree example counts two written sum calls and excludes a max call; a tail accumulator demonstrates constant active depth and ordinary budget refusal.
- [x] Inline/file/library entry points agree, existing simple queries and later-error buffering remain compatible, and complete examples explain query functions versus model functions.
- [x] Record separate consumer/core commits and update the pinned Git dependency.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse compact syntax-tree and tail-accumulator controls for frontend acceptance, native identity/type refusals and limit cleanup.

## Result

Zincite supports typed pure query functions with MiniZinc and shared brace
conditionals, direct and mutual tail calls, native Node/Declaration and nominal
contracts, and buffered read-only reports through the shared prepared evaluator.
The consumer integration is committed separately in Zincite at
`74d9122d49ef3ddcaa34afdac8cd4d9deaf48495`.

Validation:

- Independent whole review, one correction pass and a fresh focused recheck
  passed. Set, string-brace and comment-brace conditions retain both accepted
  conditional forms.
- Zirql stable and Rust 1.88 each passed 68 tests including doctests. Formatting,
  warning-fatal locked all-target Clippy, diff and zdev checks passed.
- Zincite passed 449 workspace tests and its required formatting, locked
  all-target Clippy and diff checks. Thirty public replays retain
  inline/file/library parity, call count `2`, tail total `500500`, native and enum
  identity, empty native contracts, Unknown behavior and later runtime/render
  error buffering.
- Both documentation sites passed check/build with Node 26.5.1. Their 113
  rendered outputs match the desktop/mobile, links, search and keyboard review.
- Repeated local measurements demonstrated bounded tail heap, retained argument
  admission, unchanged aliases and cleanup. Historical sample attribution and
  observed legacy projection/binding-read regressions remain explicit; the
  parser correction did not relabel old measurements.
- Shared API commits `186777e3adb2839a5ce37aaf0edc5f6dcb9d2e09` and
  `02e8a04bdf8900be5a8035b3f3452cec77a18db5` were pushed, CI-tested and fetched
  before consumer use. Zincite commit `74d9122d49ef3ddcaa34afdac8cd4d9deaf48495`
  pins `02e8a04` in its manifest and lockfile. These referenced revisions are pushed.
