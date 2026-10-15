# Execution and measured costs

The word examples build arrays and report records. In a larger request, the cost
depends on how often data is read, how much must be copied and which results the
caller retains. This chapter explains that execution model and records a
measured comparison from 2026-10-14.

Zirql prepares names and functions within one request, then evaluates them using
resolved slots and explicit continuations. Unchanged values can share storage,
while owned results and rendering still require work. Preparation also has
overhead: the experiment found slower small queries alongside faster repeated
reads of larger values.

## Runtime contracts and limits

Function signatures check actual arguments and results at runtime, including
nested elements and nominal/native identity. Preflight checks names, arity,
lexical scope, purity and available type information in every written branch.
A data-dependent shape, lookup or host operation still needs runtime checking.
`ValueType::Unestablished` means missing type information; absence, an empty
collection, host uncertainty and execution failure remain distinct.

A compatible self or mutual tail call with the same complete result declaration
reuses its activation. Arguments evaluate and are checked before replacing
caller locals. A pending addition, an incompatible return declaration, or an
outstanding typed-default check keeps the necessary continuation. Ordinary
calls consume `FunctionLimits::active_depth`; expression nesting and bounded
native callback reentry have separate guards. Every call spends work, including
an infinite tail loop. Released storage refunds no cumulative work or results.

Preparation belongs to its submitted evaluation and host context. Reusing a
parsed program against another document starts fresh preparation and admission.
It does not reuse captured handles, enum identities, shape facts or completed
results across documents. Unchanged admitted owners can retain their checks
within that request; fresh data and changed contracts require checks.

Preparation preserves the [value rules](values.md). All written branches receive
preflight checks, while an unselected default fallback does no runtime work.
Comprehensions and reducers remain eager, and sorting remains stable. Sharing
storage changes the copying cost, not those evaluation rules. The [budget
reference](runtime-limits.md) specifies the charges.

<span id="callable-worklist-storage"></span>

## Worklist storage

The native function runner also executes computational queues and identity sets.
Its loop, optional tuple patterns and ordinary conditional can implement Boolean
reachability without another evaluator or graph service. The native fixture uses
`A -> [B, C]`, `B -> [D]`, `C -> [B]`, `D -> [A]`, `E -> []`: `A` reaches `D`,
does not reach `E`, and reaches itself without traversing an edge. A language-level
expansion total shows that the unreachable search expands four distinct nodes,
despite its cycle and duplicate routes.

Queue transitions share front/rear chains; identity-set insertion shares a
bounded hash path. Retained aliases and earlier native scan states keep their
old values. This avoids a complete queue/set clone on the evaluator's ordinary
immutable slot read. Root sharing is separate from an owned
output/materialization charge. See [collection
accounting](runtime-limits.md#computational-collection-work) and [the host
contract](host-integration.md#native-computational-values).

These storage bounds and Rust API checks are separate from the timing
experiment. They establish no comparison with the older evaluator, which had no
equivalent worklist feature, and no production timing or host CLI performance
claim.

## Executable host examples

The [Zirium
report](https://zayenz.se/zirium/guides/query-language/#a-bounded-constructed-report)
and [Zincite
report](https://zayenz.se/zincite/guides/queries/#combine-functions-defaults-and-ordering)
combine a 1,000-step tail accumulator, absent-score defaults, exact string
matching and a stable report sort. Both return total `500500` and ordered rows
`gamma: 0`, `beta: 2`, `alpha: 2`, retaining the equal-score input order. Their
native recursive examples inspect [MLIR
containment](https://zayenz.se/zirium/guides/query-language/#typed-recursive-functions)
and [written MiniZinc
calls](https://zayenz.se/zincite/guides/queries/#count-written-calls-with-a-function).
Native domain meaning, source coordinates and ownership remain with each host.
The guides include complete inputs and commands for their documented
dependencies.

## Measurement method

The 2026-10-14 experiment compares the original evaluator at
`948ded4ff5157f84894129be7fdc4a4fe8cbc338` (tree
`a0ba5ad12ca4c4c0df9490b30b07dfa74bc463ab`) with the prepared evaluator at
`1d4767277d3cf7eadec4e27079aa9a570ebd9af2`. The tables call these variants
"original" and "prepared". They describe those fixed revisions, rather than
tracking the current branch. Both host guides were exercised against their
actual core pin `a278c32cace3200d496a3c0649f4cf069996e2ad`, whose product source
matches the measured prepared evaluator. Zincite source is
`dd426038ece5825fdbad80517d11a978870ea681`; Zirium source is
`fb3d0c516d0dc01b85d2a9d3070107debac6739f`.

Rust 1.98.1 release builds ran on an Apple M1 Max with 10 cores, 64 GiB and
arm64 macOS 26.6.2. One serial shuffled schedule, seed 29, contains 34 cases,
two variants per case, one separate warmup child and five measured child batches.
The 408 records contain 68 warmups, 295 measured successes and 45 measured
refusals, with no target execution failures or unfinished records. Statistics
are median (minimum–maximum) across five batches, normalized per request.
They are not thousands of independent samples. No corpus or solver was run.

Each compared pair uses matching System allocator observation with atomic
allocation, live-byte and high-water counters. The sorting driver additionally
records requested bytes. These counters add overhead in both the original and
prepared variants. These are library timings with instrumentation enabled. The
full ratio cannot be attributed to evaluator code alone. Additional
preparation/copy/frame probes run in separate prepared-source variants. Their
overhead is retained separately and is not subtracted from phase times.

The original workload cases time parsing, public preflight and evaluation, and
JSON conversion, serialization and cleanup separately. Startup, fixed fixtures
and later AST/output teardown are excluded. Binding trees are built before
timing. Exact-child `wait4` peak RSS is reported in macOS bytes converted to
MiB; it includes startup and fixtures and is not phase-specific live heap. The
small/projection fixture includes an unused pure binding. RSS alone establishes
neither reclamation nor absence of leaks.

### Original workloads

The original workloads exercise small reports, projection and repeated reads of
a saved binding. All seven completed pairs have identical outputs. The
projection retains sixteen outer names and produces three integer fields per
row. Bindings contain 128-byte strings and are read eight times. Allowances
remain 1,000,000 work, 100,000 results, 100,000 stream items and nesting 64.

| Case | Original evaluation, µs | Prepared evaluation, µs | Prepared/original median |
| --- | ---: | ---: | ---: |
| small, 1 | 1.825 (1.817–1.841) | 4.631 (4.587–4.664) | 2.538 |
| projection, 10 | 79.043 (78.869–80.790) | 49.567 (48.938–49.845) | 0.627 |
| projection, 100 | 648.237 (645.087–673.704) | 306.550 (302.408–322.208) | 0.473 |
| projection, 1000 | 6465.854 (6403.521–6561.333) | 3007.396 (2927.479–3111.521) | 0.465 |
| binding, 8 | 7.929 (7.882–8.063) | 9.356 (9.201–9.821) | 1.180 |
| binding, 64 | 35.375 (34.985–35.871) | 11.152 (11.064–11.330) | 0.315 |
| binding, 256 | 131.221 (130.600–132.433) | 17.927 (17.799–18.107) | 0.137 |

The prepared evaluator takes 2.538 times as long for the small report and 1.180
times as long for the smallest binding case. Larger inputs benefit from sharing,
but those gains do not remove the small-query overhead. The measured small
report is `{"answer": 7 * 6, "count": length([1, 2, 3])}`; the smallest binding
case reads eight 128-byte strings eight times. All paired cases keep the same
allowances.

| Case | Original parse / JSON median, µs | Prepared parse / JSON median, µs | Original peak RSS, MiB | Prepared peak RSS, MiB |
| --- | ---: | ---: | ---: | ---: |
| small, 1 | 2.239 / 0.584 | 2.300 / 0.582 | 2.297 (2.297–2.359) | 2.719 (2.703–2.734) |
| projection, 10 | 2.847 / 6.212 | 2.834 / 6.132 | 2.578 (2.562–2.609) | 2.953 (2.938–2.969) |
| projection, 100 | 4.606 / 59.700 | 3.913 / 58.658 | 2.766 (2.719–2.797) | 3.141 (3.109–3.156) |
| projection, 1000 | 15.396 / 647.688 | 14.021 / 615.396 | 4.438 (4.219–4.453) | 4.797 (4.750–4.891) |
| binding, 8 | n/a / 0.610 | n/a / 0.569 | 2.141 (2.141–2.156) | 2.453 (2.438–2.484) |
| binding, 64 | n/a / 0.690 | n/a / 0.564 | 2.188 (2.172–2.188) | 2.484 (2.453–2.516) |
| binding, 256 | n/a / 0.857 | n/a / 0.588 | 2.281 (2.266–2.312) | 2.547 (2.531–2.562) |

The raw records retain the spread for every phase. The companion 8,192-string
binding request refuses on the original work allowance and completes on the
prepared runtime with value `[8192,8192,8192,8192,8192,8192,8192,8192]`,
spending 49,556 work units. Sharing changes cumulative spending near old
thresholds. This status difference is excluded from successful paired ratios; a
refusal duration is not successful throughput.

### Recursion and repeated arguments

These typed-builder fixtures are constructed before timing. Evaluation includes
fresh Budget setup and JSON result-summary construction. Parse and render timers
are unavailable for these cases; recorded zeros must not be read as free parsing
or rendering. Function cases retain base-033 allowances: 100,000,000 work,
10,000,000 results, nesting 64, stream items 100,000; tail/report active depth is
4, and non-tail active depth is 64. Branching/infinite controls instead retain
their 20,000-work refusal allowance.

| Case | Prepared evaluation, µs | Peak child RSS, MiB |
| --- | ---: | ---: |
| tail, 100 | 297.029 (292.010–299.017) | 2.266 (2.266–2.281) |
| tail, 1000 | 2869.033 (2853.142–2883.892) | 2.172 (2.156–2.172) |
| tail, 10000 | 29498.625 (29356.084–30351.917) | 2.156 (2.141–2.156) |
| tail, 50000 | 147004.000 (146690.834–147537.791) | 2.156 (2.141–2.188) |
| unchanged, 100 | 297.454 (289.121–301.921) | 2.750 (2.641–2.766) |
| unchanged, 1000 | 2531.438 (2509.396–2617.271) | 2.578 (2.578–2.578) |
| unchanged, 10000 | 25239.834 (24953.791–25424.417) | 2.562 (2.562–2.594) |
| depth, 8 | 36.758 (35.846–39.842) | 2.266 (2.250–2.281) |
| depth, 32 | 110.658 (104.842–111.167) | 2.297 (2.281–2.297) |
| native, 100 | 771.959 (735.000–813.083) | 4.016 (4.000–4.047) |
| native, 1000 | 3963.458 (3901.208–4079.750) | 4.047 (4.000–4.078) |
| native, 10000 | 35814.292 (35609.875–36185.458) | 4.031 (4.000–4.047) |

The accumulator returns `n*(n+1)/2`. The unchanged-argument fixture passes a
record with 1,024 strings of 256 bytes plus a scalar size field through every
tail call, reading that size through a lexical local. Across all measured
lengths, temporary probes record one body preparation per request, sixteen
prepared nodes for the accumulator and fifteen for the report. Both copy zero
string payload bytes and zero outer-environment entries. The report has one
whole-record admission per request. Repeated reads still spend work.

Accumulator live peaks are two frames including the entry, eight continuations,
six runtime values and three local slots at 100 through 50,000 steps. Report
peaks are two/eight/five/three at 100 through 10,000 steps. Interpreter gauges
return to zero after success and refusal. Heap high-water above retained fixtures
is at most 4,384 bytes for the accumulator and 52,080 bytes for the report.
A batch's previous result-summary overlaps the next request by 656 bytes;
peaks are raw maxima, never divided by batch size. Cumulative allocation counts
still grow with executed work. These observations establish bounded retained
state for these cases, not constant total allocation or constant time.

Non-tail n=8 and n=32 peak at ten and thirty-four frames including entry,
with heap peaks 14,624 and 51,680 bytes. At n=128 the explicit active-depth
limit refuses with 65 total frames, 325 continuations and a 100,660-byte heap
peak. The public error category is `Other`, message
`active function depth limit exceeded`, with sixteen retained call locations
and forty-eight omitted. A later scalar operation on the same spent budget
still succeeds. Branching and infinite tail recursion separately refuse with
`WorkLimit`; a wrong result type refuses with `expected integer`. None are
included in the successful recursion table.

The native fixture retains the same public API checks used in the earlier
function experiment: nine requests, twelve body preparations and sixty-four
prepared nodes, independent of tail length. It exercises native projection,
typed native arguments, two contract refusals, native failure, bounded host
reentry and borrowed-source identity. The native timer includes the complete
fixture and assertions. Its fixed 40,104-byte heap high-water and zero
post-fixture live allocated bytes hold at 100, 1,000 and 10,000 steps. Six
copied string bytes belong to the two intentional wrong-default literals.
Live-state peaks are dominated by bounded reentry and all gauges return to zero.
This gives direct release evidence beyond RSS.

### Wide reports and stable sorting

Narrow rows contain id and integer score; wide rows add sixteen 256-byte strings
and a 283-byte common-prefix string score in the final field. Scores follow
`(id*37+11)%97`. Every result id sequence is checked against independently
stable-sorted ids; descending ties and first-input extrema are checked too.
The full `evaluate_value` path includes preflight, preparation and owned output;
fixture construction and result verification are outside timing. Allowances
remain those of base-036: 20,000,000,000 work, 10,000,000 results, nesting 64 and
100,000 stream items. These larger fixed allowances describe a separate wide
workload; they are not substituted into the original baseline comparisons.

| Case | Prepared evaluation, µs | Peak child RSS, MiB |
| --- | ---: | ---: |
| narrow_sort, 128 | 154.875 (148.167–167.167) | 2.188 (2.172–2.188) |
| narrow_sort, 1024 | 936.042 (891.959–983.542) | 3.266 (3.266–3.391) |
| narrow_sort, 8192 | 7164.625 (6953.459–7242.750) | 11.750 (11.750–12.109) |
| wide_input, 128 | 938.334 (907.125–958.417) | 4.406 (4.406–4.422) |
| wide_input, 1024 | 7150.000 (7020.625–7276.833) | 20.406 (20.391–20.500) |
| wide_sort, 128 | 1295.834 (1290.458–1324.666) | 4.406 (4.406–4.422) |
| wide_sort, 1024 | 11046.125 (10981.917–11309.708) | 20.500 (20.422–20.547) |
| wide_descending, 1024 | 11178.334 (10796.459–11289.542) | 20.453 (20.422–20.547) |
| wide_min, 1024 | 6411.334 (6278.333–6742.833) | 14.312 (14.297–14.359) |
| wide_max, 1024 | 6388.792 (6340.167–6455.000) | 14.344 (14.297–14.344) |

At 1,024 wide rows, requested heap high-water above the fixture is 8,013,748
bytes for the input-copy control, 8,055,476 for sorting and 3,111,936 for either
extremum. Sorting's required owned output remains a substantial cost. The
prepared evaluator's key probes record exactly one extraction per row. The
temporary comparison probe was attached to a different comparator and yielded no
usable count; zero in that raw field does not mean zero sorting comparisons. The
prepared driver's copy-node fields in the adapted sorting driver are unavailable
placeholders. No claim uses either field.

The historical base-036 run remains separately attributed to its saved source:
90 samples at 128, 1,024 and 8,192 rows. At 8,192 rows it observed 97,768
ascending or 97,744 descending comparisons and 8,191 extrema comparisons. The
prepared evaluator retains those ordering algorithms unchanged. Those old
observations are not relabeled as samples from the prepared evaluator or
combined with its statistics.

<span id="reproduce-and-interpret"></span>

## Evidence and limitations

The source snapshots, benchmark drivers, build logs and raw samples are kept in
a private development bundle. It also records source and binary hashes, the
exact configuration and the shuffled schedule. That bundle is not included in
this repository, so this page alone is not a complete reproduction package. The
source revisions and measurement boundaries above identify what was compared.
Earlier experiments remain separate records.

The original collector expected a `DepthLimit` category for the n=128 control;
the API reports `Other` with its explicit depth message. All twelve raw captures
(two warmups and ten measured refusals) and their original `invalid_output`
diagnosis are retained. Derived analysis validates the exact category, message,
bounded trace and successful follow-up before classifying them as refusals.
It changes no duration and performs no recollection. Original collector and
corrected analyzer versions are both retained.

Five batches on one machine provide a bounded local comparison, not a workload
survey or portable latency prediction. The allocator observer, different fixture
shapes, selected allowances and omitted host setup/rendering limit
extrapolation. The experiment has no per-phase live-memory measure and no
clean-allocator production timing. Temporary instrumentation is absent from
product source. The comparison records small-query regressions at the measured
revisions. Its bounded tail-call storage and ownership results do not establish
that every program is efficient.
