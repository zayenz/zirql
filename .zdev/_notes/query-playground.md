# A small query playground with terminal holes

## Selected scope

On 2026-10-21 the user said, “Follow your recommendations and set zdev goal
again.” This delegates the recommended minimum: a local, line-based exploration
mode for the CSV example. It does not approve unseen language syntax. This
record is a design; it changes no reader, runtime, API or consumer support and
imports no implementation tasks. The commands below are illustrative future
**tool UI**, unavailable today; they are neither shell commands nor Zirql grammar.

A CLI loop fits the existing local CSV host, located errors and buffered output.
A browser adds application, transport and editor machinery without a concrete
host need. Defer it. Keep the one-shot CSV entry point. Each submitted program
has the CSV frontend's ordered lets and final expression; no persistent bindings,
function editor, arbitrary cursor completion or evaluation on every keystroke.

## One complete interaction

The future mode loads `examples/csv/people.csv`; its display item cap is two.
`:next` takes a **complete prefix**, parses all supplied text and requires
end-of-input before displaying a terminal hole. It never removes trailing text
or salvages an earlier valid prefix. The hole and template placeholders are
presentation state outside the parser and AST, not executable expressions.
No tolerant parser, shared-reader change or editor protocol is proposed.

```text
> :next rows | filter(_.active == "yes")
draft: rows | filter(_.active == "yes") | ⟦next stage⟧
input at hole: positional sequence of declared CSV rows
fields: id: string, name: string, team: string, active: string, note: string
input-compatible templates (arguments still need checking):
  1. filter(<Boolean row expression>)
  2. map(<row expression>)
  3. count
> :choose 2 _.name
draft: rows | filter(_.active == "yes") | map(_.name)
complete query; not evaluated
> :preview
["Grace Hopper", "Ada, Lovelace"]
Showing 2 of 3 items; 1 omitted; evaluation complete.
```

`:choose` fills the selected template with the supplied expression. Its completed
query must parse and pass whole-expression preparation before preview is offered.
`:preview` reparses and preflights the complete draft, evaluates it under the
request limits, then renders the completed value. The full result is the three
strings `["Grace Hopper", "Ada, Lovelace", "Barbara Liskov"]`, in file order.
The two-item display cap never inserts `head`, changes the query or samples its
execution. The completed query is supported core/CSV syntax today; the loop is
proposed. An equivalent current CSV program can use
`let selected = rows | filter(_.active == "yes"); selected | map(_.name)`.

Starting again at that hole, `:choose 1 _.active ==` is incomplete. Report a
located parse error in the submitted argument, retain the draft for correction
and offer no preview. `:next rows | filter(_.active == "yes"` is also an error:
reject the whole malformed request and do not run `rows` or any earlier stage.
Blank input stays blank, with no query, preview, implicit `rows` or implicit
current input. A displayed terminal hole cannot itself be submitted for execution.

For a file containing only the same header, `rows` is `[]` with its declared
row schema. The `filter` and `map` bodies still receive validation, the example
preview is `[]`, and `rows | count` is `0`. Unknown fields and invalid Boolean
bodies still fail. A truly empty CSV file has no header and remains a loader
error. An untyped `[]` is different: its element type is unestablished. Show that
fact; do not invent CSV fields, claim a string element type or infer from samples.
With multiple CSV tables, use explicit names such as `people`; ambiguous `rows`
access keeps its existing error rather than selecting whichever table is first.

## Small operation inventories

Start with a private CSV table of three templates. Each entry needs a display
name, insertion template, input requirement, output rule, argument requirement,
read-only status and short help. These are tool hints; the shared reader,
`query_value_types` and full admission remain authoritative for completed queries.

| Name / insertion template | Input and output rule | Required argument / help |
| --- | --- | --- |
| `filter` / `filter(<Boolean row expression>)` | Positional sequence → same element types | Boolean body scoped to each row; keep matching rows; read-only |
| `map` / `map(<row expression>)` | Positional sequence → sequence of completed body's result types | Body scoped to each row; construct one result per row; read-only |
| `count` / `count` | Applicable collection → integer | None; count elements; read-only |

For the worked row sequence all three have compatible input. `filter` and `map`
remain incomplete until their arguments parse and pass preparation. Compatibility
with known input is not a claim that every body or later stage is valid. Separate
explicit `DeclaredType` constraints, discovered `ValueType` facts and
`ValueType::Unestablished`. The last is preparation information, not absence or
an empty value. Display samples never establish types. Keep this inventory local;
no reflection service, generic registry or new public host trait is justified.

Zirium gives a concrete future native inventory, separate from CSV support:

| Native template | Actual host transition / argument | Preview policy |
| --- | --- | --- |
| `parent`, `users` | Operations → Operations; no argument | Read-only candidates |
| `names`, `attr(<name>)` | Operations → Strings; `attr` needs a valid attribute name | Read-only candidates |
| `count` | Applicable stream → Count; no argument | Read-only terminal candidate |
| `filter(<predicate>)` | Operations → Operations; complete admitted Boolean predicate | Read-only candidate |
| `set_attr(...)`, `remove_attr(...)` | Host edits | Forbidden |
| `emit`, `json`, `format(...)`, `markdown(...)`, `print(...)` | Host emissions | Forbidden |

The native argument rules, supported input kinds and terminal restrictions come
from Zirium's validator, not names in a generic table. In
`crates/zirium/src/query/report.rs`, `stage_kind` delegates to
`program::Validator::stages`; `ReportHost::is_pure` excludes every edit and
emission above. Even a harmless-looking `json` is not a preview renderer.
Retain native owners, source coordinates, checked handles and host uncertainty;
never derive native identity or fields from displayed text. `OperationSignature`
and `checked_invoke` check trusted Rust arguments/results; they supply neither
an operation inventory nor an effect policy. CSV gains no native support here.

## Request flow, effects and limits

The future local helper takes an inspect/preview request, input paths and finite
limits. Its sequence is: admit and load inputs → parse the complete prefix or
query → discover types and perform whole-query admission → list compatible
terminal templates, or evaluate a complete read-only value → bounded display.
Inspection does not execute an incomplete query. Preview is explicit. Reuse
`CsvHost`, `Table::types`, `read_program`, `query_value_types` and
`evaluate_query_value` in private example helpers; introduce no shared API.
`evaluate_query_value` also performs its existing preparation checks.

Create one finite `Budget` per explicit request and pass it through loading,
parsing, type discovery/admission, evaluation and rendering. The current CSV
policy caps total input at 64 KiB, rows/items at 1,000, work at 1,000,000,
results at 100,000 and nesting at 64. A future implementation also chooses
finite command-length and display item/byte/depth limits. Do not reset the
budget at a stage, nested expression, host operation or renderer.

Evaluate the complete result within those bounds, then shorten its display.
Label omitted top-level items and shortened nested values/cells. Report exact
totals only when the completed value establishes them; otherwise say the total
is unestablished. A limit refusal is a located diagnostic, never partial success,
a fake total or an “evaluation complete” preview. Buffer the preview until the
request succeeds, including rendering. Presentation shortening is not sampling.

CSV exposes no native edit/emission path. Keep preview rendering in the terminal
tool, outside language emitters. A future native adapter must preflight the
whole tree for read-only admission, including unused lets, unchosen branches
and nested arguments, before execution. Hiding unsafe catalog entries is
insufficient: hand-entered complete queries must receive the same refusal.
No sandbox, rollback promise or generic effect protocol is needed for this slice.

## Bounded proposed follow-ups and documentation

1. An optional Zirql task adds the CSV local mode and private three-entry catalog,
   retaining one-shot use. Land its interface/examples in `docs/csv.md` and its
   request/type/display boundaries in `docs/csv-host.md` with the feature. Link
   from the language reference only if useful; keep tool commands outside the
   supported grammar. Use focused public behavior checks for the completed
   interaction, malformed-request refusal, header-only schema and honest display
   truncation, plus the repository's required Rust checks. Add no test matrix.
2. Separately authorized Zirium work can add a limited read-only inventory and
   preview using its actual parser, validator, purity and ownership rules. Update
   that host's query and callable/API guides with actual support and run its
   required checks. CSV/core availability does not establish native adoption.

Update embedding or host-integration guides only if actual embedding contracts
change; this design adds none. These are proposed task boundaries, not imports.
Defer browser/editor protocols, arbitrary recovery, language holes, persistent
sessions, broad discovery, effects and consumer implementation. No public guide
should advertise this mode before its implementation and documentation land.
On the actual `feature/gecode-cp` branch, `solve` is explicitly admitted optional
execution returning ordinary values, with separate ground/symbolic rules and
CP limits. A future viewer may inspect a completed ordinary result; this mode
adds no solve execution, symbolic operation, backend or branch-merge dependency.

Source review: `examples/csv.rs`, `examples/csv/people.csv`, `docs/csv.md`,
`docs/csv-host.md`, `src/reader.rs`, `src/value.rs`, `src/runtime_types.rs`;
Zirium's `query/report.rs`, `query/program.rs` and `docs/query-language.md`;
`feature/gecode-cp:docs/constraint-programming.md` and `:src/cp.rs`.
The transcript was checked against those contracts, not executed. Completion
uses `git diff --check` and `zdev check base --format json`; no Rust tests or
product execution are required for this design-only change.
