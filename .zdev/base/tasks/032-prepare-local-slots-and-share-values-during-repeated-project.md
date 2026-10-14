+++
schema_version = 1
id = "base-032"
key = "prepared-execution"
area = "base"
status = "open"
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

- [ ] One representative native report path in each consumer prepares scopes/calls once per request and executes repeated field bodies through resolved slots, retaining source locations and existing preflight guarantees.
- [ ] Repeated reads and argument-ready values borrow/share immutable data; owned outputs and necessary copies remain explicitly admitted. Dead temporaries are released without a request-wide accumulating value store.
- [ ] Runtime checks of changed values and host-dependent facts remain effective; prepared payloads cannot be reused with incompatible document context.
- [ ] Compare the named baseline cases for output/failure parity, phase time, peak memory and temporary copy/analysis evidence. Document any budget-charge compatibility change before accepting it; preserve existing defaults and entry-point contracts.
- [ ] Existing parsers and typed builders use one evaluator path; public host integration guidance describes the new preparation boundary.

## Validation

- Run the brief’s Rust and record checks; add only the focused public-behavior coverage named here.
- Run the affected consumer’s required checks and Zirql checks; demonstrate equivalent inline/file/library behavior and existing later-error output buffering.
- Reuse projection/native/value tests, adding only behavior checks for scope, host context and previously validated versus new data. Use temporary instrumentation, not permanent internal call-count assertions.
