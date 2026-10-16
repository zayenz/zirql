+++
schema_version = 1
id = "base-080"
key = "finish-pending-reader-sites"
area = "base"
status = "open"
complexity = "standard"
afk = true
priority = "normal"
blocked_by = []
+++
# Finish the pending reader-site presentation across all three projects

## Outcome

The existing reader-facing homepage and site-presentation changes are independently verified and committed before release documentation is packaged.

## Context

Existing pending tracked edits are Zirql docs/index.md, site/scripts/prepare-content.mjs and site/src/styles/custom.css, plus the preparation script in each consumer. They improve the word-frequency explanation, navigation buttons, tab height and header layout, and remove guide edit links. Root has read the complete diffs. This task explicitly adopts those five inputs; existing untracked logo explorations and notes are not part of it. No new product behavior is intended. Start read-only and return ready if the current delta meets its contract; Root allocates isolated sources before any necessary repair.

## Boundaries

- Keep current language examples accurate and preserve unrelated pending formatter work, notes and exploratory assets. Do not add new branding or redesign the sites.
- No live import/deployment or messages to quiescent chats. Apply unslop and Zayenz writing guidance to any necessary prose/comment repair.

## Done when

- [ ] The word-frequency homepage example runs and matches its displayed result; introductory prose describes the current input and queries.
- [ ] All three site checks/builds and local links pass. Root checks the changed homepage in light/dark desktop/mobile, tab switching, navigation and search, plus consumer guide edit-link removal.
- [ ] The five exact tracked inputs are independently accepted and committed in separately attributable project commits; unrelated bytes remain preserved.

## Validation

- Run actual word example, npm check/build and changed-page link checks; Root records desktop/mobile interaction proof. No product tests.
- Run diff/zdev record checks; no Rust matrix rerun for unchanged product code.
