+++
schema_version = 1
id = "base-070"
key = "csv-worked-host"
area = "base"
status = "done"
complexity = "advanced"
afk = true
blocked_by = []
+++
# Run shared Zirql queries over implicit and named CSV inputs

## Outcome

A runnable Rust example and worked guide query one implicit CSV file or multiple named CSV files through Zirql shared language execution.

## Context

The brief Language coherence, documentation and a CSV host section authorizes this third concrete host. examples/words.rs already implements ValueReaderHost and ValueQueryHost for constructed current input and an ambient binding. src/reader.rs provides read_statement_prefix, value/function/pattern readers; program.rs/functions.rs provide shared execution. docs/getting-started.md and embedding-programs.md explain existing embedding. Inspect and reuse these actual APIs for CSV rows and program composition rather than implementing another evaluator. Existing collection-order/language tasks remain separate; use accepted shared behavior available at admission and integrate later shared fixes deliberately.

## Boundaries

- Keep one library crate and a repository example executable; no separately published CSV crate, database engine, general document model, type-guessing system or CSV-specific query evaluator. CSV decoding/table loading belongs to the example host, and reusable language gaps belong in Zirql.
- Use strings for decoded header-named fields unless an existing explicit shared conversion is requested by a worked query. Do not silently infer numeric/null/Boolean types from CSV spelling. Preserve valid quoting, embedded delimiters and line breaks through an ordinary CSV decoder.

## Done when

- [x] The example runs short inline expressions against one implicitly selected CSV file and accepts multiple explicitly named CSV inputs. Names and headers have clear admission/collision rules; ambiguous implicit input and missing named tables/columns fail with useful input/query context rather than choosing a file silently.
- [x] Expose rows through existing constructed records/arrays where adequate, retaining input/header order. The example shares Zirql bindings, expressions, comprehensions, collection stages and scope; support a compact immutable-let program through existing statement/expression helpers as well as one expression. Host code only adapts CSV data, named sources and final output.
- [x] One compact people/teams fixture demonstrates single-file filtering/projection/counting and a multi-file correlated report with saved selections, using common language operations. Quoted fields are decoded correctly. A malformed row/header/unknown-field refusal and a small resource-limit refusal are located and produce no partial result output.
- [x] Input loading has a simple explicit bound, and decoding admission, parsing, binding reads, evaluation and output use the existing request budget/limits without resets. Do not add a new limits configuration framework.
- [x] The guide contains exact runnable commands, fixture paths, expected ordered JSON results and a short explanation of which code is CSV adaptation and which behavior comes from Zirql. Link runnable source and make the worked example reachable from the existing Zirql documentation site. Comments describe current contracts/reasons using the required writing skills.
- [x] Run the documented commands against the actual example and focused callable/public-behavior controls; they exercise shared parsing/evaluation rather than a host-only reimplementation. Update dependency metadata/lockfile if an ordinary CSV decoder is needed, keep its scope to the example, and validate required Rust/docs/site checks without publishing.

## Validation

- Run git diff --check, zdev check base --format json, cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace. Verify the example at the supported minimum Rust version if a new dependency changes toolchain requirements.
- Execute the single-input and multiple-named-input documented commands and compact malformed/unknown/limit controls. Use focused public behavior coverage; no timing assertions, callbacks counts or input/syntax matrix.
- Run existing documentation site check/build/link validation and inspect the worked example in the rendered site. Apply unslop and Zayenz writing-style guidance to documentation/comments; preserve unrelated homepage/site-preparation changes.

## Result

The CSV example queries one implicit or several named tables through shared Zirql expressions, immutable bindings and comprehensions, preserving CSV/header order and buffered JSON output. The worked guide is linked from the documentation site.

Validation:

- Independent whole-task review passed without corrections; the native snapshot and owned/foreign diff guards remained equal.
- Formatting, default and Rust 1.99 Clippy, workspace tests including four CSV controls, Rust 1.88 example tests/run, diff and zdev checks passed.
- All seven guide commands and ten focused refusal cases passed with exact ordered output and empty failed stdout.
- Site check/build passed; rendered desktop/mobile guide and navigation passed, with 20 pages and 966 internal links/fragments checked.
