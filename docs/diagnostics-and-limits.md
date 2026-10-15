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
| `UnsupportedNull` | Retained compatibility category for hosts that refuse null |
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
change host defaults. Existing unannotated programs retain their admission
path; the new declaration annotations charge their metadata and checks.

Result units cover new or copied scalar values, containers and their entry slots.
Keyed arrays also charge axes, entries, axis-key slots and entry-key slots.
Moving an already-admitted child into a slot does not charge its value again.
These units are accounting quantities, not a wall-clock or byte-memory guarantee.

Known range lengths, Cartesian products and byte sums are checked before
allocation. Generator candidates pay before filters. Host hooks must charge work
hidden from the core, such as scans and native materialization.

Array summaries evaluate their complete operand before reduction. New collection
construction and generator candidates remain eager. An unchanged admitted input
can be inspected through an immutable shared owner without copying its payload. Sum and both Boolean quantifiers charge one work unit
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

Default preflight visits both operands and checks available declarations and
type facts. A binding referenced only by its fallback supplies shallow kind or
explicit declaration metadata; the ordinary constructed-value API does not
scan its data until selected. Existing bindings outside default fallbacks keep
their full actual-value preflight. Native adapters must also defer fallback-only
projection, copying, member resolution and host execution.

At runtime, default charges its node and left operand, then only the selected
fallback. Presence checks charge their selected operand and Boolean result.
Lookup and membership charge key, label and nominal comparisons with the same
budget. Lookup adds an absent result on a miss; declared access copies the
selected contract metadata. Declaration wrappers, copied declarations and
constructed outputs charge before allocation. An annotation adds bounded
validation/copy nesting, and repeated re-admission does not accumulate wrappers.
Arithmetic, host and allowance failures retain their responsible locations.

Sorting evaluates its positional input once and inspects each scalar or record
key once. Field-label inspection, finite/nominal checks and key comparisons pay
work at the selector range, or the scalar helper's call range. A string comparison
charges each inspected UTF-8 byte through the first difference or common prefix.
Sort admits O(n) descriptor, merge and permutation slots before allocation and
charges descriptor writes on each merge pass and payload swaps. These are
metadata work units, with no produced-value charge. An owned array is reordered
in place; materializing a shared input pays the ordinary full output-copy costs.
Rows and declarations move intact. Descending equality still selects the left
merge input. Every comparison is fallible and stops immediately on refusal.

Extrema scan all keys once, retaining one winner without allocating sort scratch
or copying the whole input. An owned winner moves; a shared winner retains its
owner until a materialization boundary copies that selected value. Propagating a
container's declaration copies only the selected declaration and, where needed,
its selected payload. Empty extrema admit one absent value plus any retained
optional declaration. Inputs remain eager and all written branches keep
preflight checks. Later bad keys fail; no partial sorted array or winner escapes.
Host source errors and child failures retain their original locations. Existing
host limits and data policies remain unchanged.

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

## Prepared execution and sharing

Preparation charges expression/name resolution, native operand metadata and scope
setup once per evaluation invocation. Repeated rows charge executed expressions,
slot reads and shared references, current-frame/container growth, new-value
admission and host work. Known capacities and byte/entry sizes are admitted
before growth. Creating a shared owner or child-view path also charges its
allocation/metadata work.

An immutable shared read creates no deep copy, so it pays neither copied payload
bytes nor recursively produced value units. Length, reductions and field reads
can inspect that retained data directly. Materializing another owned value and
`copy_constructed_value` still scan and admit the complete copied value. Host
copies, emission snapshots and rendering keep their applicable byte/result costs.
New external data and operation results are inspected when selected. Reusing an
unchanged owner does not validate a different value or host context.

Spending remains cumulative: releasing a frame or value refunds nothing. Nesting,
per-stream ceilings and every host default remain unchanged. Repeated analysis
and avoidable copies now cost less, so a request close to an old work/result
threshold may complete, and the operation that first exhausts an allowance may
move. Limit categories and original locations remain intact. Compare successful
throughput separately from refusals when measuring this change.

Embedded primitive argument bodies share that preparation. Executing a selected
argument charges current-frame setup, references, slots, new-value admission and
native work. A `ConstructedRead` retains its immutable owner while the adapter
inspects it; `into_owned` admits a copy only when that native boundary needs an
owned value. Argument frames end with the current native invocation. Skipped
predicate leaves and default fallbacks execute no argument body, while all written
bodies retain preflight checks.


## Function calls and depth

`FunctionLimits { active_depth }` counts active user-function activations.
The caller supplies it explicitly. A compatible self or mutual tail call reuses
its activation; each ordinary call adds one. The function's final value, selected
conditional branch, selected value-match arm, final block value, successful
requirement value and untyped default fallback can be tail positions. Pending arithmetic, a typed default's remaining admission, or
a different complete result declaration keeps the required continuation.
Optionality and positional/keyed distinctions participate in that comparison.

Expression nesting still checks the written tree under the existing budget.
Native host callbacks retain their separate bounded evaluation-call guard.
Suspending a function never recursively calls the Rust value runner; synchronous
native argument callbacks can reenter it only within that host guard. Native
operations share the same context and cumulative work/results allowance.

Preparation charges signature/declaration visits, name comparisons, slots and
native metadata. A prepared unit retains that work across its written entries
and repeated executions, without retaining completed results. Native sequence
enumeration charges host scans, result containers and retained owners before
growth; grouped generators also charge their candidate product. Calls charge a visit, argument/frame slots and continuations
before growth. Runtime contracts charge scans of fresh data and declarations;
unchanged immutable constructed owners share admission within the request.
Native ownership checks remain the host's responsibility on each typed boundary.
New values, constructed annotations and explicit owned copies retain their result
charges. Releasing dead locals or returning from a call refunds nothing.

The shared continuation runner also handles function-free expressions. Its
continuation and operand metadata can increase work spending for a small query,
so requests near a work threshold can fail earlier. Limit defaults, categories,
branch demand and eager collection evaluation retain their contracts.

`FunctionError` keeps the original core or native error in `error`. Its `calls`
contains at most sixteen active locations, innermost first, with function names
capped at 64 characters; `omitted_calls` counts any remaining active locations.
A reused tail activation records its current call. This
context is captured once on failure and does not retain completed calls or a
chain of deferred return checks. Cleanup needs no remaining allowance; subsequent
operations on the same budget retain the spending but start without active
function or host guards.


## Pattern and guard work

Pattern preparation charges visits, names, static labels, comparisons and local
metadata. Runtime matching charges each visited pattern, inspected label or
literal, failed arm, staged binding and retained child path before growth.
Guards use ordinary expression work plus a Boolean selection check. A failed
pattern or false guard still spends its work. They never reset the request's
allowances or turn a host, admission or resource failure into a fallback.

A whole typed argument is admitted at its written argument range. Parameter
pattern mismatches use the failed parameter pattern; block binding mismatches
use their binding pattern. Guard failures keep the guard or failing child range,
and native failures retain their original source coordinates.

Destructuring shares selected constructed/native children. It neither copies
ignored rest fields nor rescans an already admitted parent to bind each child.
An unadmitted native subject receives host admission before matching. Native
views and owned-result boundaries charge their actual inspection and copy work.
Pattern metadata can increase preparation work for requests near an earlier
threshold; allowance defaults and result-copy charges are unchanged. Finished
match/guard work releases its subject before the selected result runs. Compatible
tail calls discard completed block/arm cleanup when replacing the activation.


## Named accumulator work

Fold and scan prepare the named step once. Each item uses ordinary whole
argument admission, parameter patterns, function execution and result admission.
The fixed state contract is checked before the step frame returns, so a bad
actual state keeps its active function context, including after a compatible
tail call. Finished steps release their frames; sequence length does not add
active call depth. Nested calls and native reentry retain their separate guards.

Preparation, source selection, each step, immutable owner sharing, temporary
slots and scan snapshots spend the same request budget. Native sequence hooks
also charge their own scans and construction. Fold retains its source and
current state; scan additionally retains the requested post-step snapshots.
Constructed snapshots receive owned copies at the existing public result
boundary. Releasing a state refunds no allowance, and failure releases partial
output and pending work without requiring spare budget.

Step lookup and signature errors point to the step name. Initial admission uses
the initial expression's range, while item/container failures use the items
range. Errors within a step retain their body or native source locations.


## Loop work

Callable loop initialization, each body visit, transfer search, next-state
operand and contract admission spend the same active allowance. Metadata for
state slots, staged values, fixed contracts and continuations is admitted before
growth. Completed iterations release their locals and pending work, with no
refund of cumulative work or results. A fixed-width loop can run beyond the
active function-depth ceiling; non-tail calls, expression nesting and native
reentry retain their separate limits. An endless `Continue` receives an ordinary
located work-limit error. Operand, contract and budget failures release staged
state and restore active evaluation state; a later operation may use any
remaining allowance.


## Computational collection work

Queue/set construction, type/context checks, fresh element admission, native
identity, hashing, collision equality and storage growth use the active budget.
Queue links, retained element owners, tuple slots and trie child slots charge
before allocation. Pop and append preserve the existing per-collection item
ceiling; work and produced storage are cumulative and releases refund nothing.

A queue shares concrete front/rear chains. Append adds rear links; pop takes a
front link. When the front is empty, rotation charges each inspected/shared/moved
rear entry and each new front link. A single advancing queue rotates each batch
once, giving amortized constant work per entry. Retaining a branch can repeat a
rotation when that branch is advanced separately; each repetition is charged.
Long queue chains release iteratively, including after resource failure.

An identity set uses a randomized 64-bit hash and at most sixteen nibble steps.
Insertion copies the bounded path under aliases and charges copied child slots;
full-hash collision leaves retain exact tokens and charge each equality check.
Duplicate insertion retains the existing roots. It does not rebuild the set.
There is no promise of constant work for a deliberately colliding bucket.

Unchanged queue/set admission checks the current owner stamp and whole element
metadata. It does not copy or rescan the remaining collection. Selected pop
fields retain their native owner through explicit tuple views. Native container
inspection keeps the ordinary nesting guard. A stale context, element mismatch,
unsupported serialization or exhausted allowance remains an error, distinct from
an empty pop or a false membership result.
