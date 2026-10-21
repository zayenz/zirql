+++
schema_version = 1
id = "base-119"
key = "integrate-shared-json"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-113", "base-118"]
+++
# Replace consumer data serializers with shared Zirql JSON rendering

## Outcome

Zincite and Zirium delegate ordinary-data JSON traversal and output bytes to Zirql, retaining only native projections and host envelopes.

## Context

Read brief.md section Consolidate language machinery and data rendering (2026-10-21), including its integration and validation rules. Use the shared JSON task APIs. Zincite comprehension.rs tagged/tagged_key and program.rs report_json/admit_json/BudgetWriter, Zirium query/comprehension.rs to_json and query/program.rs to_json/serialize_json are starting points. Mixed native reports currently recurse through host result containers; make the common container bridge concrete using existing views, leaving native leaves and completeness/document fields with hosts.

## Done when

- [ ] Both consumer libraries and CLIs use shared ordinary/tagged codecs and budgeted writing, including data nested in native report containers; replaced recursive constructed serializers and byte writers are removed.
- [ ] The shared boundary handles container traversal so host code supplies native-leaf projections and envelopes without another serializer. Existing native identity/owner and unsupported computational-value checks remain enforced.
- [ ] Zincite tagged output and completeness metadata, Zirium ordinary JSON and document/NDJSON envelopes, ordering, pretty formatting and whole-request buffering remain compatible; newly representable nominal/keyed data follows the shared contract.
- [ ] Host callable/Python conversion paths use the shared codec where they emit JSON, without changing native Python handle APIs. Host output docs and examples identify shared formats and local envelope fields.

## Validation

- Compare existing library/CLI JSON fixtures for both hosts, include one mixed native/constructed report and a later failure after an emitter. Exercise affected Python conversion tests.
- Run the brief’s Rust checks in every changed Rust repository and affected host/Python checks; git diff --check and zdev check base --format json for records. Record separate consumer commits and accepted Zirql dependency revisions.
