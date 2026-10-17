# Zirql documentation: language and embedding

Planning round, 2026-10-15.

## Editorial direction

Present Zirql as a language for data transformation and analysis that an
application can embed. The opening should show a useful expression, its input
and its result. Explain the Rust interfaces after the reader has seen what the
language does.

Use a list of words as the main teaching model. Begin with constructed strings
and records. Introduce native occurrences when a task needs source identity or
navigation. Zincite and Zirium are adapter examples; their commands, dialect
semantics, installation and user workflows belong on their respective sites.

Retain the selected logo, light/dark SVGs and existing palette. Use code and
results to establish the front page's character. Avoid feature slogans and
claims about future capabilities. The writing follows unslop and
zayenz-writing-style.

## What the review found

The previous landing page described a Rust library and concentrated on a
three-stage pipeline. It did not show the grouping, ordering and constructed
reports supported by the current common reader. The setup guide built `6 * 7`
as a Rust expression tree, so it did not explain how to run the language shown
on the front page.

The language pages mix user-visible rules with Rust interface details. Some
text still called `one` and comprehension syntax proposals. The design page
mixed available mechanisms with hypothetical word-host syntax. The integration
page contained complete Zincite/Zirium command walkthroughs.

## Structure

| Section | Reader's question | Pages |
| --- | --- | --- |
| Front page | What can I write? | Adjacent pairs, selection and summary tabs, each with input and output |
| Start here | How does this example work? | Word analysis; set up the word model in Rust |
| Language reference | What does an expression mean? | Overview; values and comprehensions; functions and control flow; pipelines; programs; diagnostics and limits |
| Implementation reference | How do I embed and extend it? | Overview; host interfaces; value evaluation; program construction; budget APIs; word-model extensions; adapter examples; execution and costs |

Keep existing guide URLs during this pass. Add entry pages for the two
references and keep the Rust API link separate from the conceptual explanation.
Language usage should link to implementation details only when that distinction
helps the reader understand a boundary.

## Changes prepared in this round

- Reframe the front page around data analysis and supply three switchable word
  examples using current common-reader syntax.
- Rewrite the word tutorial around filtering, record construction, frequency
  counts, stable ordering and empty input.
- Add a complete `examples/words.rs` host and connect the setup guide to it.
  The host exposes one constructed binding, rejects native queries, uses one
  budget and leaves rendering to its caller.
- Add language and implementation reference entry pages with links to the
  existing detailed contracts.
- Reduce the consumer guide to adapter choices and links to the consumer sites.
- Replace the mixed-status design page with explicitly hypothetical word-model
  extensions. Correct stale reader and cardinality claims in pipeline/program
  introductions.

This round changes documentation, site presentation and its runnable example.
It introduces no core language behavior and does not publish a site build.

## Detailed reference pass

The second pass separates the language rules from the Rust interfaces:

- Values and comprehensions cover literals, access, operator precedence,
  arithmetic, absence, built-ins and collection semantics.
- Functions and control flow cover conditions, local bindings, patterns,
  declaration syntax and recursion. Fold, scan, loops and worklists are identified
  as callable forms that need a host frontend.
- Programs and native data use word occurrences to explain bindings, parameters,
  projections, captures, paths and output policy.
- Value evaluation and budget APIs preserve the ownership, type, preparation and
  accounting contracts previously mixed into language pages. The host reference
  starts with the constructed word adapter before introducing native hooks.

The source baseline is `4eaaafb7cfc112dbc0673fd67007a16f1f8e8ee2`.
Changes were prepared in an isolated copy and checked against that baseline
before integration, preserving concurrent framework work. These are documentation
scopes, not imported base tasks; the active framework task records are unchanged.

Front-page feedback also changed the opening. Add a short motivation before the
examples, remove the large Word analysis action and lead with adjacent word
pairs. The runnable example uses the same default expression. In the tutorial,
grouping retains occurrence positions; plain counting points to native tally.

## Review and publication

For the local pass, build the site and check the tab controls with keyboard and
pointer, both themes, narrow widths, navigation and search. Execute the displayed
word expressions with the example and inspect their results. Run the repository's
required formatting, lint, test and zdev checks without adding product tests.

A publication pass must use a reviewed source revision, import the complete
static build through the personal site's established process, and check the
live paths. A source push alone does not update zayenz.se. The current request
is a planning and rewrite round; publication remains a separate step.
