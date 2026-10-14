+++
schema_version = 1
id = "base-032"
key = "prepared-execution"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-029", "base-030"]
+++
# Prepare local slots and share values during repeated projections

## Outcome

Repeated projection executes a prepared expression with resolved local slots and shared immutable inputs instead of repeating body analysis and copying unchanged values.

## Context

Use the computation baseline and runtime type admission contract. Inspect evaluate_projection/evaluate_query_value, local-name reads and Bindings::with_local. Read the brief’s preparation, ownership, budget and compatibility sections before changing these paths.

## Boundaries

- A focused behavior-preserving runtime change; no JIT, general optimizer, cross-request result cache or new lazy collection semantics.

## Done when

- [x] One representative native report path in each consumer prepares scopes/calls once per request and executes repeated field bodies through resolved slots, retaining source locations and existing preflight guarantees.
- [x] Repeated reads and argument-ready values borrow/share immutable data; owned outputs and necessary copies remain explicitly admitted. Dead temporaries are released without a request-wide accumulating value store.
- [x] Runtime checks of changed values and host-dependent facts remain effective; prepared payloads cannot be reused with incompatible document context.
- [x] Compare the named baseline cases for output/failure parity, phase time, peak memory and temporary copy/analysis evidence. Document any budget-charge compatibility change before accepting it; preserve existing defaults and entry-point contracts.
- [x] Existing parsers and typed builders use one evaluator path; public host integration guidance describes the new preparation boundary.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse projection/native/value tests, adding only behavior checks for scope, host context and previously validated versus new data. Use temporary instrumentation, not permanent internal call-count assertions.

## Result

Prepared located projection bodies and embedded native arguments once per invocation, resolved local slots, shared immutable reads, and integrated both consumers through pinned Git revisions while preserving demand, context, admission and owned results. Core source 75ec373; Zincite 45b416a; Zirium 411fbfb. Retained baseline comparisons document improved repeated work with small-query overhead and slightly higher RSS.

Validation:

- Fresh independent whole-task PASS with no findings at Wcb82e2bba6fad5c6; retained core correction history and equal framework/all-three-repository comparisons before separate integration commits.
- Required stable and Rust 1.88 checks, warning-fatal Clippy/rustdoc, workspace tests, doctests, formatting, diff and zdev checks passed; all affected host checks passed.
- Fresh isolated CPython 3.11-3.14 extension builds, Ruff, ty and pytest passed, each 171 passed and two existing skips; all 1825 original environment, extension, lock and symlink paths match retained hashes.
- Library, parsed and typed native reports, inline/file CLI behavior, later-error buffering, native argument demand/context and DATA atomic/refusal controls passed; doubled DATA passed MiniZinc instance-check-only.
- Exact retained five-batch projection/binding cases passed parity and measurement checks; copy/preparation and bounded live-ownership evidence retained privately, with observer and RSS limitations and separate work-limit controls.
- All three guide-site checks/builds passed; Root actual desktop/mobile review passed for final hosted-origin outputs, navigation, search and links. Both consumers were committed separately and pushed with frozen source hashes.
- Zirium Quality CI 38562924304 passed all twelve jobs for exact integration commit 411fbfb; core CI 38559224040 passed both jobs for 75ec373. Zincite has no workflow run for its integration commit.
