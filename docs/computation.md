# Execution and costs

A word analysis is cheap enough that copying its input may go unnoticed. With
thousands of report rows or repeated function calls, the size of the values
matters as much as the number of expressions. This chapter explains where Zirql
spends work and what to measure in an embedding application.

## What happens in a request

Before execution, Zirql checks names, arity, scope, purity and available type
information in every written branch. It prepares names and functions for that
request, then evaluates the selected branches. Data-dependent shapes, lookups
and host operations still need runtime checks.

Preparation belongs to the submitted evaluation and its host context. A parsed
program can be reused against another document, but preparation and admission
start afresh. Captured handles, type identities and completed results do not
carry over between documents.

For an application, the full cost also includes loading its data and handling
the result. Pass the same budget through those steps and the evaluator. The
[budget reference](runtime-limits.md) specifies the core charges and the work
the host must account for.

## Sharing and copying values

Within a request, unchanged values can share storage. Reading a large binding
or passing it through recursive calls need not copy all its string payloads.
Fresh values and changed type contracts still require checks, and producing an
owned result or rendering it requires additional work.

This matters when a query builds a wide report. Sorting its rows is only part
of the cost; retaining all their fields in the returned value can cost more.
Measure the result your application actually needs, including its conversion
or rendering.

Sharing preserves the [value rules](values.md). Comprehensions and reducers are
eager, sorting is stable, and an unselected default fallback does no runtime
work. Releasing storage does not refund cumulative work or result charges.

## Recursive calls

A compatible self or mutual tail call with the same complete result declaration
reuses its activation. An addition waiting for the recursive result, an
incompatible return declaration or a pending typed-default check needs a
continuation instead. Ordinary calls consume `FunctionLimits::active_depth`.
Expression nesting and native callback reentry have separate guards.

Every call spends work. Tail recursion can avoid growing the active call stack,
but an infinite tail loop still exhausts its work allowance. See
[functions and control flow](functions-and-control-flow.md#recursion-and-limits)
for the language rules and [function limits](runtime-limits.md) for accounting.

<span id="callable-worklist-storage"></span>

## Worklist storage

Queues and identity sets support traversals that track pending and visited
native objects. Queue transitions share front and rear chains; identity-set
insertion shares a bounded hash path. Earlier values remain valid when an
updated queue or set is created.

This storage sharing avoids a complete collection copy on an ordinary immutable
binding read. It does not remove the charge for an owned output. The
[computational value contract](host-integration.md#native-computational-values)
and [collection accounting](runtime-limits.md#computational-collection-work)
describe the integration requirements.

<span id="original-workloads"></span>
<span id="measurement-method"></span>
<span id="recursion-and-repeated-arguments"></span>
<span id="wide-reports-and-stable-sorting"></span>

## Measure the whole integration

Measure parsing, validation and evaluation separately from loading input and
rendering results. Use representative input sizes and retain the same outputs
and resource allowances when comparing implementations. A refused request is
not a faster successful request.

The [2026-10-14 measurement report](execution-measurements.md) compares the
original and prepared evaluators at fixed revisions. Preparation made its
smallest report slower, while repeated reads of larger bindings benefited from
sharing. Those measurements used allocator instrumentation on one machine;
they do not predict latency in another application.

<span id="reproduce-and-interpret"></span>
<span id="evidence-and-limitations"></span>

The report includes workload definitions, timing tables and limitations. Its raw
samples and drivers are in a private development bundle, so it is not a complete
public reproduction package.
