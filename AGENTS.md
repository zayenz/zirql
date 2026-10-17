# Repository guidance

## Keep the framework small

Every abstraction needs a concrete use in Zirium or Zincite. Prefer one Rust
library crate, direct evaluation and typed native results. Check whether each
interface and test earns its maintenance cost.

## Planning and implementation

Read [.zdev/base/brief.md](.zdev/base/brief.md) before planning or implementing
framework work. It owns shared language rules, host boundaries and testing.
Read its linked comprehension contract when working on expressions or data.

Use zdev for durable work in `.zdev/base/`. The base area follows `main` and
planning records are tracked. `TASKS.md` is generated; do not edit it by hand.
Use the selected task's boundaries and completion conditions. Planning does
not authorize implementing the framework or changing a consumer repository.

Zirql must remain independent of both consumer crates. Host adapters retain
native handles and source coordinates; rendering is an explicit boundary.
Surface parsers may differ, but supported comprehensions share one evaluator.
Keep MiniZinc model facts, MLIR dialect semantics and source-edit safety with
their hosts. Preserve unrelated changes in every repository touched by an
approved integration task.

For documentation and metadata changes, `git diff --check` is sufficient.
Run `zdev check base --format json` when changing zdev records.
For Rust code changes, use:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Integration tasks also run the affected host's required checks. Use focused
public-behavior coverage from the brief; avoid tests of internal call counts,
generic test matrices and timing assertions. Track `Cargo.lock` when created.

## Releases

This is a personal hobby project with no established external dependents.
A bug fixed in a follow-up release is an acceptable trade-off. Keep release
work proportionate to that context.

1. Update the intended versions, necessary lockfiles, short release notes and
   affected installation/documentation text.
2. Use the existing CI and the publication commands in
   [RELEASING.md](RELEASING.md). Publish Zirql before consumers that require
   the new version.
3. Publish the authorized tags and release notes. Import the latest applicable
   docs build through [site/PUBLISHING.md](site/PUBLISHING.md) and use the
   existing deployment process. Reuse authorization already given.

A successful build and successful existing publishing action are sufficient
release assurance. For manual publication, successful CI and the normal publish
command are sufficient. Successful deployment is sufficient for the docs site.
Report the versions and URLs, then stop.

Trust checks already performed by CI and publishing jobs. Do not repeat them
locally or through additional reviewers. Fresh installation environments,
manual archive/checksum audits, extra test matrices and repeated consumer/API
exercises are not release gates. Version, dependency pin and documentation
changes do not reopen accepted code reviews or language testing.

Extra validation needs a concrete failure, known defect or explicit user
request. Investigate that problem only, then continue the normal process.
This release policy overrides heavier validation checklists in older briefs,
tasks and release procedures.
