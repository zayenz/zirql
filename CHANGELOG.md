# Changelog

## 0.1.2

- Support insertion-ordered string sets, typed empty sets and `set<string>`
  signatures, with unordered membership equality and lexicographic sorting.
- Tally arrays of string sets into first-seen `key`/`count` records while keeping
  existing string tally dictionaries. Retain inferred types for empty results.
- Add a runnable neighbouring-word set example and document its result, limits
  and host serialization boundary.

## 0.1.1

- Avoid whole-container copies when selecting retained collection values.
- Index string deduplication while preserving first occurrence and declared types.
- Share immutable CSV tables through the existing host hook, reducing copies
  and peak memory in the runnable example and embedding guide.
- Clarify runtime work accounting. Public APIs and default limits are unchanged.

Paired measurements with three measured repetitions per implementation found
16,000 distinct declared strings deduplicated in 62 ms instead of 26.35 s
(426 times faster). Repeated lookup in 400-entry dictionaries and keyed arrays
was about 40 and 89 times faster, respectively. Counting one million four-column
CSV rows took 1.69 s instead of 2.52 s for the complete buffered run, with peak
process memory reduced from 1.57 to 1.05 GiB. These comparisons isolate each
change; they do not measure their combined effect or predict every workload.

## 0.1.0

Zirql's first non-prerelease version provides one Rust library for shared query
execution in Zirium, Zincite and smaller application hosts.

- Common readers and located Rust builders support expressions, immutable
  bindings, comprehensions, dictionaries, insertion-ordered sets and sparse
  keyed arrays with retained index domains.
- One reader accepts common `for/in/if` and MiniZinc `pipe/where`
  comprehensions, grouped and binding generators, natural records, MiniZinc
  conditionals, Boolean spellings, multidimensional indexing, matrices,
  starting-key tails and quoted identifiers/comments. Scoped expression readers
  and a narrow domain-call hook keep generic query parsing in Zirql.
- Scoped filters accept named and unnamed arguments, Boolean expressions,
  lexical captures and permitted pure calls. Native predicate adapters use the
  same expression scopes while retaining their existing subjects and leaves.
- Shared destructuring covers lets, typed parameters, predicate arguments,
  comprehensions and value matches. Positional lists accept exact patterns or
  a final discarded `..`; predicate `_` retains its whole subject. Mismatches
  fail with locations except that value-match arms try the next pattern.
- Checked scalar operations, absence/defaults, stable ordering, grouping,
  reducers and requirements compose with native projections and pipelines.
- Typed pure functions support destructuring, guarded matches, state loops,
  fold/scan and bounded recursion with compatible tail-call reuse. Native queues
  and identity sets support worklists while preserving retained aliases.
- Native APIs provide ordering, set algebra, maps, correlated existence,
  fixed points, captures and bounded shortest paths. Hosts retain object
  ownership, identity, source coordinates, completeness and effect policy.
- A request budget covers parsing, validation, evaluation, copying and host
  work. Type, arithmetic, purity and resource failures retain their locations.
- The crate includes runnable word and CSV hosts, CSV fixtures and reader
  guides. CSV accepts one implicit input or multiple named inputs, immutable
  bindings and one final expression. It keeps cells as strings and does not
  accept named function declarations.

Rust 1.88 is the minimum supported version. Zirql installs no executable.
Computational queues and identity sets have no direct rendering or data
serialization. Functions have no closures, imports, overloads or stored function
values; external parameters accept only `int`, `string` and `bool`. Host-specific
syntax, data loading, rendering and source edits remain with each application.
The 0.1 series does not promise a stable 1.0 API.

## 0.1.0-alpha.5 (2026-10-13)

Published nominal members, typed sparse keyed arrays and keyed comprehensions
through the five budgeted value APIs. This prerelease predates the functions,
control flow and collection additions included in 0.1.0.

## 0.1.0-alpha.4 (2026-10-13)

Published budgeted scalar expressions and positional comprehensions, with
validation, free-name discovery, type discovery, evaluation and copying APIs.

## 0.1.0-alpha.3 (2026-10-12)

Published iterative flat left-associated native set sequencing while retaining
the grouped and recursive nesting limits.

## 0.1.0-alpha.2 (2026-10-12)

Published the native program API.

## 0.1.0-alpha.1 (2026-10-12)

First usable prerelease of the shared core.

## 0.0.0-prerelease (2026-10-12)

Published and yanked to reserve the crate name. This version is not a usable
registry dependency. The usable alpha releases remain available.
