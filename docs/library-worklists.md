# Queues and native identity sets

These shared forms run through the function evaluator with a host supplying
computational values, type resolution, ownership and identity. They are not
ordinary serialized collections. Saving or updating a queue/set preserves
existing aliases. Foreign or stale owners, incompatible contracts and exhausted
limits fail. The word/CSV examples do not install this worklist host support.
See [computational collections](value-runtime.md#native-computational-collections).

## queue

`queue(items)`, `queue<T>(items) -> queue<T>`

Build a FIFO from an admitted positional sequence, preserving element order.
`queue<int>([2, 3])` puts `2` at the front in a supporting host. An explicit
type or context establishes the element contract when an empty input cannot
infer it: `queue<Operation>([])` is a native example. Items must satisfy the
whole contract. An unestablished empty queue fails. `queue<T>` also declares
a queue type; it does not turn the queue into an iterable array.

## pop_front

`pop_front(queue<T>) -> optional (T, queue<T>)`

Return the front item and remaining queue, or absence when empty. With a queue
containing `2, 3`, match `present((item, remaining))` to read `2` and the queue
containing `3`. For example, `absent(pop_front(queue<int>([])))` is `true` in a
supporting host. No existing alias is consumed or advanced. Non-queues fail.

## push_back_all

`push_back_all(queue<T>, items) -> queue<T>`

Append an admitted positional sequence in its order. In a supporting host,
`push_back_all(queue<int>([2]), [3, 4])` yields front-to-back `2, 3, 4`.
An empty append preserves the queue's contents and element contract. Every item
must meet that contract and native context; the original queue remains usable.

## identity_set

`identity_set<NativeType>() -> identity_set<NativeType>`

Create an empty set of one explicitly named native kind. For a host exposing
`Operation`, `identity_set<Operation>()` starts with no members. Use `insert`
and `item in seen`; there is no iteration, structural serialization or set
algebra. Native identity includes the owner and other host distinctions, so two
objects with equal printed names can differ. Missing or non-native element types
fail. `identity_set<T>` also declares its type.

## insert

`insert(identity_set<T>, native_item: T) -> identity_set<T>`

Insert one admitted native identity, retaining existing aliases. If `op` is a
valid `Operation`, `op in insert(identity_set<Operation>(), op)` is `true`.
Repeated insertion is idempotent. The item's kind, owner and validity must match;
this does not insert into a constructed structural set.
