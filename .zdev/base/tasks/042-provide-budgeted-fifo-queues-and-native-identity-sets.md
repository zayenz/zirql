+++
schema_version = 1
id = "base-042"
key = "native-worklist-collections"
area = "base"
status = "open"
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

- [ ] queue<T>, construction, optional pop_front and push_back_all preserve FIFO order and native values. identity_set<T>, membership and idempotent insert use host-provided semantic identity consistently.
- [ ] Typed signatures/locals admit queue and native identity-set values, including optional nested pop results. Explicit empty element contracts, stale/foreign handles and document/diff-side distinctions are enforced.
- [ ] Operations preserve retained aliases while allowing efficient unique-owner state transfer. Pop-result extraction, pop and unchanged-value admission do not copy or rescan the entire remaining queue per operation.
- [ ] Frame/container/identity/hash/equality work and growth spend the active budget; direct rendering/data serialization fails with a located error, while library results can retain computational values.
- [ ] A callable queue/visited-set control processes the example’s cycle and duplicate routes with each node expanded once and demonstrates identity differences independent of printed names.

## Validation

- Run the brief’s Rust and record checks. Extend existing public native/value controls only for the named new behavior; no internal call-count or timing assertions.
- Check FIFO/empty pop, duplicate identity insertion, retained aliases, incompatible identity/context and resource refusal. Use temporary increasing-worklist measurements to expose whole-collection copies and retained temporaries.
