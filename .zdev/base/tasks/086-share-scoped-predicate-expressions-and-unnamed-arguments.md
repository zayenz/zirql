+++
schema_version = 1
id = "base-086"
key = "general-constructed-filter"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "high"
blocked_by = []
+++
# Share scoped predicate expressions and unnamed arguments

## Outcome

Predicate expressions support named and unnamed arguments through shared language machinery, including constructed filtering and the CSV worked example.

## Context

The user identified starts_with-only constructed filtering as a release blocker, then required underscore as an unnamed argument and the same predicate language everywhere in the z-family. people | filter(p => p.active == "yes") and people | filter(_.active == "yes") must agree with [p for p in people if p.active == "yes"]. Read src/reader.rs ValueSuffix/read_constructed_filter/read_value_suffix/bare filter, ValueReaderHost scoped hooks, native filter predicate_or, NativeMatch where and paths(to=...), and host root, diff and scalar edit predicate entry points that need the same reusable reader contract. Read existing comprehension evaluation/type/free-name/budget APIs and focused shared-reader/value tests. Read the Scoped predicate expressions section in the area brief for the inventory and subject/scope rules. They include native filter/root, path targets, match guards, Zirium diff predicates and Zincite scalar-element edits, not only constructed filtering. Consumers adapt existing native subjects and operations to the shared predicate tree in the following integration task; they currently call the restricted hostless helpers. Use the proposed p => expression and implicit _ expression forms without general first-class function values.

## Boundaries

- Use shared value/comprehension parsing, scope/type validation and evaluation for general Boolean expressions; do not add a second predicate evaluator. Preserve existing native predicate leaves, domain meaning, native/nominal identity, source offsets, order and cumulative parsing/execution/result/nesting limits. Shared syntax must be usable by all inventoried host predicate positions.
- Keep existing collection shapes; element types already admissible to the relevant comprehension/native subject path must work. Add no first-class function system, optional selector/relation overloads, compatibility framework or generic syntax/test matrix. Existing convenient domain/prefix forms may remain where simple.
- This task owns shared predicate/reader contracts, core/CSV code, focused behavioral coverage and current-language documentation. It does not edit consumers or publish/deploy. Root supersedes the old core candidate and makes the reviewed actual Git revision available for the following integration.

## Done when

- [x] Constructed filter accepts named p => predicate and implicit _ predicates with field comparisons, Boolean composition, lexical captures and the same permitted pure expressions/calls as comprehension predicates. Explicit source and current-input forms share implementation and host-aware name/call resolution; order and supported element types agree with comprehension selection.
- [x] The common predicate contract covers the complete brief inventory, including native filter/root, paths targets, match guards, diff predicates and scalar-element edit predicates, through shared parsing/evaluation hooks without replacing domain leaves or host effect policy. The existing subject of each position is explicit; ordinary if/require/match/comprehension Boolean guards reuse predicate expression rules without becoming new callback overloads.
- [x] Each predicate argument scope binds _ to that position's existing subject; an optional named argument denotes the same subject. Nested predicate scopes rebind _, named outer captures remain usable, and _ does not leak outside its predicate scope or change wildcard-pattern meaning. Typed-empty validation, non-Boolean/type/purity failures and finite budgets follow the existing shared rules with original locations.
- [x] Focused existing-style public tests demonstrate row selection equivalent to a comprehension, captures/calls, nested unnamed scope, representative non-filter predicate use and empty/failure/budget behavior without a matrix. CSV simple row selection uses filter; packaged implicit/named reports and a located refusal with empty stdout run successfully.
- [x] Required core Rust/MSRV/docs and site/link checks pass. Language/API/release notes describe current behavior and host integration contracts. Root independently verifies/commits/pushes the source and supersedes old candidate hashes before final acceptance.

## Validation

- Run required fmt, fatal workspace/all-target Clippy, workspace tests, existing Rust 1.88 tests and warning-fatal docs/doctests. Add only a few public behavior controls for this exhibited defect, including nested _ versus named outer capture and a representative native non-filter predicate contract.
- Run CSV implicit/named reports from packaged example sources: named/unnamed filter equals comprehension output, captures, empty input and located predicate failure with empty stdout. Keep CSV named-function declaration limits explicit.
- Inspect every changed public helper/hook and its caller contract against the predicate inventory. Run affected site check/build/links and proportional desktop/mobile guide checks; package updated sources/guides. No publication.

## Result

Shared named and unnamed predicate arguments now use the existing value/comprehension runner, including active native calls and scoped domain arguments. Independent whole review and one correction/recheck verified Boolean grouping and lexical argument precedence; consumer integration and destructuring follow their dependency tasks.

Validation:

- Stable and Rust 1.88 default-stack workspace tests, fmt, fatal all-target Clippy, fatal docs/doctests and zdev/diff checks passed.
- Verified 60-file draft package matches canonical sources and guides; extracted CSV named/implicit/captured filters agree with comprehensions, empty/refused requests retain locations and empty stdout.
- All 1222 site link/asset/fragment targets and 21 generated API links passed; Root desktop/mobile, themes, menu and search review passed.
