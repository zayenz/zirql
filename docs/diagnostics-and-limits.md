# Diagnostics and limits

A failed arithmetic expression needs a query location. A failed source access
needs the host's source location. Resource failures use the location of the
operation that exhausted the request budget.

## Query and source locations

`QueryError` holds a message and `Location::Query` or `Location::Source`, each
with a half-open UTF-8 byte range. Query ranges identify the original query
expression. Source ranges refer to the host's retained source; the host keeps
any document identity needed to interpret them.

`EvaluationError::Core` retains a shared `QueryError`.
`EvaluationError::Host` retains the host's native error type. Do not convert a
runtime error into `Truth::Unknown`: Unknown is a domain predicate answer,
not an error-recovery mechanism. Hosts also retain incompleteness information
separately from ordinary failures.

A terminal-stage transition, duplicate binding or known type mismatch should
fail before domain effects. A missing field or position with unestablished type
facts can fail when accessed at runtime. An overflow fails at the arithmetic
expression. A limit refusal identifies the active query or supplied source
location and returns no partial constructed value.

Parameter declaration errors use their original name or type range. Missing
required inputs point to the declaration name; incompatible values point to its
type. Duplicate, extra or malformed caller arguments use the request range and
identify the parameter in the message. A host retains its inline/file source label
instead of inventing a query byte range for command-line arguments.

## Request allowances

| Field | Meaning |
| --- | --- |
| `work` | Request-wide charged work, including nested execution |
| `results` | Aggregate produced/copied value and slot units for program budgets |
| `stream_items` | Ceiling for one native stream |
| `nesting` | Logical nesting allowance, capped at `MAX_NESTING` (64) |

Keep the same `Budget` through admission, validation, nested execution and
copies. `Budget::for_program(ProgramLimits { ... })` enables aggregate results.
Legacy `Budget::new(Limits { work, items, nesting })` retains the per-stream
`items` ceiling and leaves aggregate result charging inactive. The value APIs
still charge work under a legacy budget. Zirql supplies no new default limits;
the host must choose values appropriate to its requests.

## What uses the allowance

Work pays for expression, value and type visits, validation metadata, comparisons,
known collection allocation, generator candidates and copied UTF-8 bytes.
Parameter admission charges declaration lookup metadata, raw argument bytes,
name copies, scalar values and copied string bytes before allocating them.
Its full-set validation completes before domain execution. Empty declarations
and inputs spend nothing, so literal-only requests keep their existing costs.
A value snapshot scans the full size and invariants, admits the result units
and copied bytes, then constructs the copy.

Result units cover new or copied scalar values, containers and their entry slots.
Keyed arrays also charge axes, entries, axis-key slots and entry-key slots.
Moving an already-admitted child into a slot does not charge its value again.
These units are accounting quantities, not a wall-clock or byte-memory guarantee.

Known range lengths, Cartesian products and byte sums are checked before
allocation. Generator candidates pay before filters. Host hooks must charge work
hidden from the core, such as scans and native materialization.

Array summaries pay the full operand's materialization, candidate and copy
costs before reduction. Sum and both Boolean quantifiers charge one work unit
per inspected element, including elements after a decisive Boolean. Length
charges one work unit for reading the present-value count and converting it to
`i64`. Each summary adds one aggregate result unit for its scalar, including
empty arrays.

Require preflight visits both expressions and charges its literal message bytes.
A false requirement charges diagnostic-copy bytes before allocation and fails
at the requirement call. Its skipped value has no runtime arithmetic, access,
construction or host-check charges; referenced binding projection and actual-type
discovery can still spend work before runtime. A true requirement moves its
already admitted value without another result or copy charge. Arithmetic errors
retain their child ranges, and limit errors retain the active operation range.
Emission snapshots and rendering charge again under the same budget.

Spent allowances are not refunded after success or failure. Active scope and
nesting state are restored on both outcomes, so a failed operation does not
leave the budget stuck inside a nested call.

## Effects and output

Parsing and validation are read-only. A pure expression cannot edit or emit.
The host defines transactions and rendering behavior for explicit effects.
A CLI can buffer output until the full request succeeds. Library emitters and
live edits do not gain automatic rollback from the core; callers must use the
host's documented effect contract.
