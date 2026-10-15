# Diagnostics and limits

A query can handle a missing label with a default. A division by zero still
fails. Zirql distinguishes absent data, unknown domain facts and errors so that
handling one does not hide another.

| Outcome | Example | What the query can do |
| --- | --- | --- |
| Absence | `lookup({}, "word")` | Use a default or match presence |
| Unknown predicate | The host cannot resolve a source reference | Retain the host's uncertainty rules |
| Error | `1 div 0`, incompatible types or exhausted work | Fail with a location |

`default` handles absence. It does not catch arithmetic or resource errors.
Negating an unknown predicate keeps it unknown. A filter retains only known
true answers; a required native match guard or path target refuses an unknown
final answer.

## Query and source locations

Query diagnostics point into the original expression. A missing field points
to its access, a failed arithmetic operation to that operation, and a limit
failure to the work that exhausted the allowance. Source failures retain the
host's source coordinates and document identity.

A request can fail before it executes. Preflight checks reject undefined names,
known incompatible types, duplicate bindings and effects in pure contexts. A
condition that is always false does not hide those errors. Missing data or a
division by zero in an unselected runtime branch is not evaluated.

When a function fails, the host can report a bounded context of active calls.
Tail-call reuse records the current activation rather than every completed
recursive call. Diagnostic wording and presentation belong to the host.

## Request allowances

The embedding application chooses finite parsing and execution limits:

| Allowance | What it bounds |
| --- | --- |
| Work | Cumulative evaluation, validation, scans and other charged operations |
| Results | New or copied values and container slots under a program budget |
| Stream items | The size of one native stream |
| Nesting | Logical expression nesting, with a library maximum of 64 |

A nested query shares the remaining allowance. A new input stream, function
call or comprehension does not start a fresh budget. These are accounting
limits, not promises about elapsed time or peak memory in bytes.

## What uses the allowance

Comprehensions visit every candidate, including candidates rejected by a filter.
Reducers receive eager arrays. Sorting inspects keys and uses temporary storage;
grouping builds records and member arrays. Copying a saved result or rendering
it can spend additional work and result units.

A short expression can therefore do substantial work. For example, two
independent generators over `words` visit every pair, while a generator over
positions with an access to the next position visits only adjacent pairs.
Choose the expression that matches the question you intend to ask.

Spent work and results are not refunded when a value is released or an operation
fails. Active scope and nesting state are restored after a failure. An exhausted
request fails; it does not quietly return a shortened report or partial path
search.

## Function calls and depth

The application also sets an active function-depth limit. A compatible tail call
can reuse an activation; a call with pending work needs another one. Each call
still spends work. An endless tail recursion eventually fails its work allowance.
Expression nesting and native callback depth remain separate limits.

## Effects and output

Parsing and validation are read-only. Pure expressions and functions cannot edit
or emit. A host with explicit output or editing operations defines their effect
and rollback policy. A buffered command may publish only a complete result;
a live emitter may already have produced output before a later failure.

<span id="prepared-execution-and-sharing"></span>
<span id="pattern-and-guard-work"></span>
<span id="named-accumulator-work"></span>
<span id="loop-work"></span>
<span id="computational-collection-work"></span>

Adapter authors should read [budget and diagnostic APIs](runtime-limits.md)
for error categories, charging rules, callback obligations and cleanup. The
[execution guide](computation.md) explains preparation, sharing and measured
costs.
