# Diagnostics and limits

A failed arithmetic expression needs a query location. A failed source access
needs the host's source location. Resource failures use the location of the
operation that exhausted the request budget.

## Query and source locations

`QueryError` holds a message and `Location::Query` or `Location::Source`, each
with a half-open UTF-8 byte range. Query ranges identify the original query
expression. Source ranges refer to the host's retained source; the host keeps
any document identity needed to interpret them.

`QueryError::category` gives hosts a stable `QueryErrorCategory` for the failures
they translate:

| Category | Failure |
| --- | --- |
| `WorkLimit` | Request-wide work allowance exhausted |
| `ItemLimit` | Native stream item ceiling exceeded |
| `ResultLimit` | Aggregate produced/copied result limit exceeded |
| `UnsupportedNull` | Shared value reader refused a null literal |
| `UnknownPredicate` | Required native match guard or path target remained Unknown |
| `Other` | Any failure without a dedicated category |

The producer assigns the category independently of the message. Host adapters
match categories for their specific diagnostic wording and forward other messages
with their original locations. The enum is non-exhaustive; adapters must retain
an informative fallback for future categories too.

`QueryError::query(range, message)` and `QueryError::source(range, message)`
construct `Other` errors. A producer that knows the failure uses
`.with_category(category)` without changing the message or location. Matching
message text never assigns a category: even `"work limit exceeded"` remains
`Other` when passed to a general constructor. Direct struct construction must
supply the public `category` field, and equality compares it along with the
message and location.

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

Declared runtime contracts use the supplied admission location. A checked native
call uses each argument's written range for argument refusals and the call range
for arity, declaration and result refusals. Nested mismatches identify the field,
tuple position, sequence element or keyed cell in the message. Positions are
one-based; keyed cell positions follow the stored sparse entry order. Native
owner, source and operation failures remain `EvaluationError::Host` with their
original error type and coordinates. A contract error never becomes Unknown.

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

Declared admission charges declaration visits, actual-value/type discovery,
metadata slots and label bytes, contract comparisons and diagnostic bytes.
It uses the existing nested evaluation guards. Native adapters charge their
member and ownership checks under that same budget. `checked_invoke` charges a
call visit, admits arguments, runs the trusted operation and checks its result
without resetting the allowance. An operation can therefore succeed internally
and still fail result admission when its work has consumed the budget.

Admission borrows the value and adds no materialized result charge. Fresh
operation outputs still require producer admission before allocation; explicit
copies keep their existing charges. Immutable structural checks can remain
valid for the same value and contract, but these APIs retain no checked-value
cache. Native context changes, new values and data-dependent access still need
runtime checks. Declaration facts do not expand external parameter types or
change existing value-entry-point costs or host defaults.

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

Grouping materializes its positional rows before scanning them. Row visits,
selector-label comparisons, typed key comparisons and nominal metadata checks
charge work. New group records, the outer array, each `items` array and their
entry slots charge work and aggregate results before allocation. Each new
group copies its key and the `key`/`items` labels; existing rows move without
another value-copy charge. A depth scan checks the extra group record and items
array around each moved row. Type discovery pays for schema metadata and labels.
Missing fields and incompatible keys use the selector range; input-kind errors
use the grouping call. Child query or source failures keep their locations.

Keyed entries and exact pairing materialize each operand once. Ordered axis and
full-key comparisons, nominal metadata checks and type facts charge work.
Entries admits the positional array, records, tuple keys where needed, slots
and copied `key`/`value` labels. Exact pairing admits the keyed result slots and
two-element tuple cells, retaining admitted left axes and keys. Cell values move
without another deep copy. Depth scans check the added wrappers; later snapshots
and rendering spend the same allowance. Input-kind, alignment and operation
limit failures use the helper call range. Child and host-source failures retain
their responsible locations. Every executed operand receives the host value
check before the helper can consume it.

Native matches require aggregate program result limits. Every clause and guard
branch is preflighted before subject enumeration. Dependent clause calls and
native field expressions use the active logical nesting allowance; Boolean
wrappers keep that depth active through native leaf evaluation. Candidates pay
before later filters. Capture rows, field slots, labels, native copies and host
materialization share the same request budget. Exhaustion fails the match and
returns no partial row vector.

Native paths require aggregate program result limits too. Preflight visits the
edge and all target branches before source enumeration. Source and neighbor
scans, identity comparisons, traversal slots and predecessor records charge
before growth. Witness reconstruction admits steps, native copies, field slots,
labels and result rows before allocating them. The per-stream ceiling also
bounds retained traversal nodes, witness steps and result rows. Hosts charge
their native materialization and later rendering with the same active budget.
A final Unknown target fails at its target range; host source errors retain
their locations. A depth-bound absence gives no witness within that bound,
and exhaustion fails the request instead of truncating it.

Spent allowances are not refunded after success or failure. Active scope and
nesting state are restored on both outcomes, so a failed operation does not
leave the budget stuck inside a nested call.

## Effects and output

Parsing and validation are read-only. A pure expression cannot edit or emit.
The host defines transactions and rendering behavior for explicit effects.
A CLI can buffer output until the full request succeeds. Library emitters and
live edits do not gain automatic rollback from the core; callers must use the
host's documented effect contract.
