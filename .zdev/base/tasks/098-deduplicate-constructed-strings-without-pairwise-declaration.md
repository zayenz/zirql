+++
schema_version = 1
id = "base-098"
key = "indexed-string-unique-performance"
area = "base"
status = "done"
complexity = "standard"
afk = true
priority = "high"
blocked_by = []
+++
# Deduplicate constructed strings without pairwise declaration validation

## Outcome

Constructed unique retains first occurrences of admitted strings with indexed membership and avoids rederiving declaration facts on every compared pair.

## Context

The same analysis measured 1k/4k/16k distinct CSV strings at 107/1660/26934 ms in the public evaluation API. Concatenating an empty string before unique produces identical plain string bytes and lowers the 16k case to 1554 ms, still quadratic. src/value.rs unique_value scans retained entries and equal_values re-admits declared scalar types per pair; the sampled profile confirms these costs. Start with unique_value, scalar_pipeline_elements, equal_values and existing declared-value admission tests. Read the brief collection-order and budget contracts and docs/runtime-limits.md. Baseline and plain-string controls are under /private/tmp/zirql-perf-20261017.

## Boundaries

- Optimize the demonstrated string case with ordinary collections and direct compaction; no public key type or general indexing framework. Other scalar kinds may keep their existing path. Preserve first occurrence, exact string equality, annotations, declarations, empty-array behavior, heterogeneous-input refusals and all nominal/float behavior.
- Do not remove required one-time validation of externally supplied or malformed declared values. No user-visible syntax, dependency, native-host algorithm or consumer changes; no release or deployment.
- Keep request allowances unchanged, admit scratch before growth, charge key inspection and retained output, and document changed spending. Avoid timing tests or assertions about internal calls.

## Done when

- [x] Declared and plain string arrays deduplicate in first-seen order with the same retained values and declarations; existing scalar and refusal semantics remain valid.
- [x] A compact focused public case covers declared strings, duplicates and retained ordering where existing tests do not already suffice; no generic matrix is added.
- [x] Matched temporary 1k/4k/16k declared and plain string sweeps validate identical outputs and show that distinct-string performance no longer follows the prior pairwise scan. Evidence and limitations are retained locally.
- [x] Required source checks pass and runtime-limits documentation describes the actual index/allocation work.

## Validation

- Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace.
- Run git diff --check; coordinator runs zdev check base --format json.
- Use focused existing public-behavior tests and temporary matched performance measurements; no timing assertions, internal counters, new benchmark framework or broad test matrix.

## Result

String unique uses one local hash index and direct row compaction, preserving first declarations and order. Scratch and key costs are charged before growth; other scalar kinds retain existing equality.

Validation:

- Independent Astra/high whole-task verification passed; snapshot W150dc791711099b6 remained equal. No public API or dependency changes; implementation fits existing Rust ownership and located-error patterns.
- Formatting, warning-fatal workspace Clippy, workspace tests, staged/unstaged diff checks and zdev check passed.
- 48 matched samples passed with identical outputs using the frozen original CSV host. At 16000 distinct strings median evaluation declined from 26348.548 to 61.884 ms with declarations, and 1537.882 to 70.225 ms for plain strings. The 1000/4000/16000 sweep shows near-linear candidate scaling; enlarged temporary probe allowances remained fixed.
