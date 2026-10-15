+++
schema_version = 1
id = "base-042"
key = "native-worklist-collections"
area = "base"
status = "done"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-031", "base-033"]
+++
# Provide budgeted FIFO queues and native identity sets

## Outcome

Shared computational queue and identity-set values support efficient immutable worklist transitions with native handles.

## Context

Read the selected collection contracts and worked graph. Build on base-030 admission, base-031 optional values and base-033 callable native value handling. Existing sorted scalar/nominal sets are a different value kind; retain their public behavior.

## Boundaries

- No changed scalar-set ordering, identity-set iteration/algebra, debug serialization, generic graph service or persistent global cache.

## Done when

- [x] queue<T>, construction, optional pop_front and push_back_all preserve FIFO order and native values. identity_set<T>, membership and idempotent insert use host-provided semantic identity consistently.
- [x] Typed signatures/locals admit queue and native identity-set values, including optional nested pop results. Explicit empty element contracts, stale/foreign handles and document/diff-side distinctions are enforced.
- [x] Operations preserve retained aliases while allowing efficient unique-owner state transfer. Pop-result extraction, pop and unchanged-value admission do not copy or rescan the entire remaining queue per operation.
- [x] Frame/container/identity/hash/equality work and growth spend the active budget; direct rendering/data serialization fails with a located error, while library results can retain computational values.
- [x] A callable queue/visited-set control processes the example’s cycle and duplicate routes with each node expanded once and demonstrates identity differences independent of printed names.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Check FIFO/empty pop, duplicate identity insertion, retained aliases, incompatible identity/context and resource refusal. Use temporary increasing-worklist measurements to expose whole-collection copies and retained temporaries.

## Result

Added bounded native FIFO queues and identity sets with shared immutable storage, context-aware admission, borrowed selections and host-aware scalar scan completion.

Validation:

- Independent whole review and approved focused correction reviews pass; stable fmt/check/strict Clippy and stable/Rust 1.88 workspace suites pass (11 pipeline, 40 native, 46 value controls and 8 doctests each).
- Callable FIFO, aliases, identity/context, graph worklist, native scans, borrowed ignored payload and scalar scan controls pass; qualified historical allocation observations support unchanged storage bounds without final-source timing/allocation claims.
- Site checks and actual guide UI/link/search/focus evidence match staged task inputs; diff and zdev checks pass. Unrelated docs work remains unstaged and preserved.
