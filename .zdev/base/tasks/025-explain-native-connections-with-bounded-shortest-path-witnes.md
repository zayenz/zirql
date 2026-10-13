+++
schema_version = 1
id = "base-025"
key = "bounded-path-witnesses"
area = "base"
status = "open"
complexity = "advanced"
afk = true
priority = "normal"
blocked_by = ["base-022"]
+++
# Explain native connections with bounded shortest path witnesses

## Outcome

One shared traversal operation returns deterministic native shortest witnesses for Zirium SSA-use and Zincite proven-reference connections.

## Context

Read the explanatory-path research, native capture result contract, Zirium users/navigation docs and Zincite transitive_references semantics. Reachability already exists; this slice retains endpoint/step records that explain connections using the native record/projection path now adopted by both hosts. Read the brief's concrete commands and expected witnesses: Zincite referenced_declarations follows proven targets in existing reference order, with declarations as steps and one connection per edge.

## Boundaries

- Zirql owns bounded traversal, visited/predecessor accounting, stable witness choice and result construction. Hosts enumerate typed edges in documented order and provide native identity and limitations.
- Return one shortest witness per reachable source/sink pair within max_depth edges, including a zero-edge witness when the source satisfies the target. No all-path search, query optimizer, dataflow proof or edits.

## Done when

- [ ] A common paths(edge, to=predicate, max_depth=n) form and typed API produce native source/sink/steps records; steps include both endpoints, and equal-length alternatives follow host edge order.
- [ ] The brief's Zirium users example yields [arith.muli, arith.addi, func.return]; its Zincite referenced_declarations example yields native declarations [a, b, c] at depth two and no witness for c at depth one. Neither host duplicates traversal.
- [ ] Cycles, depth zero and exhaustion retain explicit behavior; depth-limited absence is not claimed as global disconnection. Incomplete edges preserve host limitations, and endpoints never cross document/diff-side identity boundaries implicitly. An Unknown target predicate causes a located refusal.
- [ ] Both hosts provide compact inline and saved-program examples with the same native/library outcomes and explicit limits, documenting the different meanings of their edges.

## Validation

- Use a small branching/cyclic graph to observe shortest-path tie order and depth bounds, then one public control per host plus an incomplete-edge case from existing semantic fixtures.
- Run required Zirql and affected host checks. Exercise budget refusal without timing assertions or a graph benchmark campaign.
