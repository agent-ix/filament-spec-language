---
id: FR-205
title: "Define the protocol subject and its canonical state key"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-425
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-828
    type: depends_on
---
# FR-205: Define the protocol subject and its canonical state key

## Description

FSL SHALL build a temporal claim's **protocol subject**, a model subject
(FR-125) together with one checked protocol clause of its package and the
clause's bindings, as FSpec FR-425 defines it, and SHALL key each protocol
state by FSpec FR-425's state key, so that two states with equal keys are
one state (ADR-027 PS-1 to PS-8, FO-3, AE-2).

## Use case

A verification operator asks whether a claim holds over a protocol that
forks two branches over one cell. They name the model subject, the
protocol, the object its `over` parameter binds and the model each static
role plays. Every engine and replay reads one definition of what a protocol
state is, so a counterexample's state keys mean the same thing to the
model checker and to replay, and a loop that forks again reaches a state it
has already seen instead of a new one.

## Inputs

- `ProtocolSubject` (`fsl_analyze::model_check`, ADR-029): a `ModelSubject` (FR-125);
  `protocol`, the declared identity of one protocol clause of the subject's
  checked package; `over`, the object reference bound to the clause's `over`
  parameter; `roles`, a binding for each static role on a model type; and the resolved
  memory model of each outermost `parallel` (FR-219, ADR-027 SE-4): the
  request's selection when present, else the source default, `Sc` when the
  clause declares none. Each replicated role's population universe is the subject's
  universe for that population (ADR-027 RR-1).
- The checked protocol clause S3 produces (FR-218).

## Outputs

- `ProtocolSubject` and, for a protocol state, its `ProtocolKey`: FSpec
  FR-425's form of FR-181's `{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…}`
  with `queues` and `roles` keyed by protocol instance ordinal.
- The initial protocol states of the subject.
- For a protocol state and a checked item, its cycle key (Behavior, "Cycle
  key").

## Behavior

### Subject

- The subject builder SHALL build `ProtocolSubject` from the model subject,
  the protocol clause, the `over` binding, the static role bindings, the
  capacity binding, the replicated role universes and the resolved memory models, as FSpec FR-425
  states. The protocol clause's `terminal` member (FR-211) and its
  `scheduling` member (FR-212) are part of the checked package, so the
  subject's obligation identity (ADR-013 O-09) binds them. The identity
  binds the resolved memory model of each outermost `parallel` (FR-219),
  and the source `memory` clause only through it.
- When the request names a protocol that is not a protocol clause of the
  package, the subject builder SHALL refuse `invalid_runtime_input`/
  `invalid-value`, naming the protocol.
- A static role on a whole model (FR-218, `role w on M`) is bound by
  construction: its binding is the subject's model subject, which holds
  every model the package selects, each recorded by its `model_root` node (FSpec FR-361, FR-425), and
  its `roles` entry is `{"type":"model-binding","selection":"<identity>"}`
  with the identity of the package `M` selects (FSpec FR-425-AC-13). Two such roles on two
  aliases are each bound. The request supplies no binding for it.
- When a static role on a model type has no binding, a binding names a model
  the role does not admit, or the request supplies a binding for a role on a
  whole model, the subject builder SHALL refuse
  `invalid_runtime_input`/`wrong-role-mapping`, naming the role.
- When the `over` object is not an object of its declared type's population
  universe, the subject builder SHALL refuse `invalid_runtime_input`/
  `invalid-value`, naming the parameter and the object.
- The subject carries the capacity binding `capacities` of the clause's
  `symbolic(<name>)` queue capacities (FSpec FR-785). An entry naming no
  capacity of the clause refuses `invalid_runtime_input`/`unknown-member`
  and a repeated name `duplicate-member`, before the requires-bound
  pre-check; a name with no entry is an unbounded root, reported
  requires-bound naming it and each of its channels. A system built without
  that pre-check, such as a counterexample replay (FSpec FR-433), refuses an unbound name
  `invalid_runtime_input`/`missing-member`.
- A refinement whose abstract side is a protocol takes the binding from the
  request's abstract protocol binding, with its `over` object: a defect
  refuses as above and an unbound name is requires-bound, before any state
  is explored. A supplied trace's `finite-replay` takes its subject from the
  `analyze` request, whose subject carries no capacity binding, so a
  protocol with a symbolic capacity is an unbounded root there and the item
  takes the requires-bound row.

### State and key

- `ProtocolSystem` (FR-215) SHALL hold each protocol state as FSpec FR-425
  states it, with threads, paths and ordinals, parallel instances, binders,
  iteration counters, queues, role instances and the memory component, and
  SHALL remove parallel instances, binders and finished protocol instances
  in the steps FSpec FR-425 and ADR-027 PS-4, PS-5 and AE-3 name.
- `ProtocolKey` SHALL be FSpec FR-425's state key, with `queues` and
  `roles` keyed by protocol instance ordinal, each map sorted by JCS bytes,
  and the memory component as the member the memory model supplies, empty
  under `Sc`. The key SHALL also hold, as FR-425 states: for each
  compensation thread, the registration it compensates; for each protocol
  instance in which a step of a template's commit node has occurred, that
  node; and, for each compensation thread, its attempt record in a state if
  and only if the thread rests at its retry point, its attempt count is below
  the template's `attempts n`, the template's retry relation reads its
  `earlier` binder, and no step of the template's commit node has occurred in
  its instance (ADR-027 PS-8, ST-11). That attempt record is the retry
  relation's `earlier` input only. Separately, `control` SHALL hold, for each
  compensation registration, its attempt history: one entry per dispatched
  attempt (`cattempt`, `cfail` or `ctimeout`), in dispatch order, with the
  attempt's record, its outcome and its `retry_of` (FR-893), kept from the
  attempt's step until the instance finishes and removed with the
  registration (ADR-027 PS-8, AE-3). Two states that differ in any dispatched
  attempt's record, outcome or `retry_of` therefore have different keys,
  whether or not the retry relation reads `earlier`.
- Each `queues` entry SHALL be exactly its **message** (the message
  occurrence of FR-894: instance, send node, thread path and that node's
  occurrence ordinal on the path), its **delivery** index and its payload;
  it carries no separate send node or thread path, which the message
  already holds. `bounds` SHALL hold `message-ordinal`, the next occurrence
  ordinal of each send node on each path, and `next-delivery`, each
  message's next delivery index, both kept until the instance is removed,
  after the sending thread has finished and after every copy has left the
  queue (FSpec FR-425-AC-15). The indices follow three rules: the copies one
  `send` step dispatches (one, or `d` for an interval send's `count` step or
  continuation step) take consecutive delivery indices from the message's
  next index, a rejected or lost copy included, and a `duplicate` takes the
  next index; the next index advances by the number of copies the step
  dispatched; and the occurrence ordinal advances only when the send
  completes, so every continuation step of a pending interval send is the
  same message. A branch path is reused once its parallel instance is
  removed (PS-2), so keeping the ordinal until the instance is removed means
  the reused path's next `send` takes the following ordinal, and no message
  identity is reused within an instance, queued or already received. Two
  states that differ only in their message or delivery history therefore
  have different keys.
- `bounds` SHALL hold `business-effect-ordinal`, the next occurrence ordinal
  of each effect node on each path (FSpec FR-425-AC-16). An `effect` node's
  `event` step takes the ordinal as its effect occurrence key's ordinal
  (FR-894) and advances it by one; the entry is present from the node's first
  `event` step on the path and kept until the instance is removed, after the
  thread on that path has finished, so a reused path's next `effect` step
  takes the following ordinal and no effect occurrence key is reused within
  an instance. Two states that differ only in their effect history therefore
  have different keys.
- Thread, instance, registration and nesting counts SHALL have no ceiling
  of their own; the run's method bounds and resource limits (FR-209, FR-215) are
  the only bounds.
- The subject SHALL have FSpec FR-425's initial protocol states: under
  `on origin`, instance 0 settled from the start of `run` (FR-206); under
  `on each`, no instance (FR-209).

### Cycle key: message, delivery and effect identities the checked item does not read

Message ordinals, delivery indices and business-effect ordinals (FR-894) are
identity labels, and the state key keeps them until the instance is removed
so that none is reused. So the `next-delivery`, `message-ordinal` and
`business-effect-ordinal` bounds only grow while the instance lives. Without
a further rule a loop of `duplicate` and `lose` steps, a `repeat` with no
maximum around `send` and `receive`, or a `repeat` with no maximum around an
`effect` node, never returns to an equal key, even though nothing the claim
can see has changed.
FSL SHALL therefore compare protocol states for coalescing and loop closure
by a **cycle key**, which keeps an identity wherever the checked item reads
it and canonicalises it wherever the item does not.

- **Read sets.** An obligation reads a step's identities only through a
  step identity read (FSpec FR-828, FR-894 "Step identity reads"):
  `filament::protocol::message(N)`, `filament::protocol::delivery(N)` and
  `filament::protocol::effect(N)`. For a checked item (a model-check item,
  FR-125, with its clause), the read sets are computed statically at S3 from
  the step identity reads the item's clause holds, in its formula, its
  captures and its activation guard: a channel is in the item's **message
  read set** when the clause holds a `message(N)` or `delivery(N)` read
  whose node `N` sends or receives on that channel; it is in the item's
  **delivery read set** when the clause holds a `delivery(N)` read whose
  node receives on it; and an effect node is in the item's **effect read
  set** when the clause holds an `effect(N)` read of it. The delivery read
  set is part of the message read set, because a delivery identity names its
  message. A read counts whether or not the item's evaluation reaches it, so
  the sets are a sound over-approximation of what the item can tell apart. A
  `send` or `receive` binder holds the payload only, a capture of the
  protocol the trigger record, and a protocol guard or constraint cannot
  hold a step identity read (FSpec FR-828), so the obligation is the only
  thing that places a channel or an effect node in a read set.
- **Canonical form.** For a protocol state `s` and the item, `canon(s)` is
  `s` with, in each protocol instance, the following changes. A send node
  and path with a pending interval send (FSpec FR-426, FR-509) keeps its
  ordinals, messages and indices unchanged.
  1. **Message ordinals**, for each send node `n` and path `p` whose channel
     is outside the message read set: the `next-delivery` entry of every
     message of `(n, p)` with no queued copy is removed; the `m` messages of
     `(n, p)` that have a queued copy are renumbered `0, 1, …, m - 1` in
     queue order of their first queued copy (a `fifo` or send-ordered
     `unordered` queue's order; on any other `unordered` queue, ascending
     payload bytes, then ascending number of queued copies), in every queue
     entry and `next-delivery` key that names them; and the
     `message-ordinal` bound of `(n, p)` is set to `m`, and removed when `m`
     is 0. Messages tied in that order have equal payloads and equal numbers
     of copies, so swapping their ordinals changes no byte of the result.
  2. **Delivery indices**, for each message of a channel outside the
     delivery read set: its queued copies are renumbered `0, 1, …, k - 1` in
     queue order (on an `unordered` queue that is not send-ordered,
     ascending JCS bytes of the entries with the delivery member omitted;
     copies tied there are byte-identical apart from the label), and its
     `next-delivery` bound is set to `k`, and removed when `k` is 0.
  3. Each changed `unordered` queue is re-sorted by its entries' JCS bytes
     (PS-7).
  4. **Business-effect ordinals**, for each effect node outside the effect
     read set: its `business-effect-ordinal` entry on every path is removed.
     No other member of the key names an effect occurrence key, so no entry
     is renumbered; this is step 1 with `m` = 0 for every effect node.
  `canon` changes nothing else: payloads, send nodes, thread paths,
  positions, capacities and every other member of the key are kept. The
  **cycle key** of `s` is the `ProtocolKey` of `canon(s)`. Inside the read
  sets the full identity stays: a channel in the message read set keeps its
  message ordinals and `message-ordinal` bounds, one in the delivery read
  set also keeps its delivery indices and every message's `next-delivery`,
  and an effect node in the effect read set keeps its
  `business-effect-ordinal` entries.
- **Uses.** EN-1 (ADR-027 TS-6) SHALL key each product state by the cycle key
  of its protocol state and store `canon(s)` as the state, so two states with
  equal cycle keys are one state. A closure certificate's protocol states are
  cycle keys (FR-338). A lasso over a protocol subject closes when the cycle
  key of its loop's last post-state equals the cycle key of its entry state
  (FR-217, FR-329).
- **Counterexamples keep concrete labels.** A counterexample's steps and
  post-state keys SHALL be those of the concrete behaviour from the initial
  state, never relabelled: EN-1 re-executes the found path from the initial
  state, taking at each position the step whose identity equals the path's
  step after the inverse of that position's relabelling and whose post-state
  has the path's cycle key. So a reported `duplicate` carries its own
  message's next index on every round of the loop, and a reported `send` its
  own next ordinal, neither used by an earlier step of the counterexample,
  as FR-894 states; only the closure test reads the cycle key.
- **When the item reads the identity.** On a channel in a read set the cycle
  key keeps what the item reads. A loop that takes a `duplicate` on a
  channel in the delivery read set, a `send` on a channel in the message
  read set, or an `event` step of an effect node in the effect read set, and
  does not finish the instance (ADR-027 AE-3) never closes,
  because the bound it advances grows and no step but the instance's removal
  lowers it. EN-1 then gives bounded evidence only: a run that reaches its
  horizon with no counterexample settles ADR-018 V-5
  `BoundReached{depth: k}`, a run limit settles V-7, and neither settles
  `proved` while such a step stays enabled. A refutation by a finite prefix
  (a safety violation, an undefined evaluation, a deadlock) and a lasso that
  closes without such a step are refutations as before. Such a loop is never
  offered or accepted as a lasso: FR-217 and FR-329 refuse it as a loop that
  does not close.
- **Completeness limit.** Read labels are kept raw, so an "at most once"
  claim over a `repeat` with no maximum around a read step is never `proved`
  by EN-1, even when it holds; it settles with bounded evidence. Renumbering
  the read labels that no live capture of any obligation instance holds would
  still be a bisimulation and would let such claims be proved; that
  refinement is not specified here (FSpec FR-828 "Completeness limit"), so
  the cycle key keeps every read label and never gives an unsound proof.

#### Decision: canonicalise unread identities, keep read ones

`canon` is sound because the relation "equal cycle keys" is a bisimulation of
`ProtocolSystem` that preserves everything the item checks. Let `s` and `t`
have equal cycle keys. Outside the read sets they differ only by a
relabelling of the ordinals of their queued messages and the indices of
their queued copies, and in identity history: the `next-delivery` entries of
messages with no queued copy and the values of the `message-ordinal` and
`next-delivery` bounds, and the `business-effect-ordinal` bounds of effect
nodes outside the effect read set. A label is used only as an identity
token. ST-4 picks an entry by send node, payload and position, and ST-5 by
entry and capacity. A message with no queued copy enables no step, since
`receive`, `duplicate` and `lose` each need a queued copy and a pending send
is left unchanged by `canon`; no step's enabledness reads an effect ordinal
(ST-2 reads the attempt's binder, not the ordinal). A `send`, `duplicate` or
effect `event` step takes a fresh label: the send node's next ordinal on its
path, the message's next index, or the effect node's next ordinal on its
path. The
matching steps in `s` and `t` may take different labels, but each label is
greater than every label of the same kind still in use, and `canon` gives
the new message or copy the same position-based rank in both post-states, so
the post-states have equal cycle keys. So for each step of `s` there is a
step of `t` of the same kind, thread, node and channel, the same counted
status (PB-3), and the same payload and binder value, whose identity differs
only in a label of an unread channel or effect node and whose post-state has the same cycle
key, and the converse holds. The two states carry the same model state, so
every atom has the same truth (PB-2); they enable the same fairness classes
and a matching step is taken in the same class, since channel steps are in
no class (PA-2) and a `send` or `receive` step belongs to its thread's class
whatever its labels. The item reads no identity of these channels or effect nodes, so no
obligation sees the difference. Hence a closed loop of cycle keys unrolls
into an infinite concrete behaviour whose letters, distances and fairness
readings equal the lasso's at every position, and every concrete behaviour
projects onto a path of cycle keys. A verdict and a lasso read over cycle
keys are exact for the concrete protocol. Erasing an identity that the item
reads would merge states the obligation tells apart, so the read sets keep
those identities, and the claim gets bounded evidence instead of an unsound
proof or lasso.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-205-AC-1 | ADR-027 §7's `Fill` subject (universe `{c}`, `c.v = 0`, `k` bound to `c`, `w` a role on the whole model `M`, supplied no binding) builds. Two subjects that differ only in the `over` binding, or only in the clause's `terminal` member, have different obligation identities. Naming a protocol absent from the package refuses `invalid_runtime_input`/`invalid-value`; supplying a binding for `w`, and leaving `v` unbound in a variant with `role v on M::Cell`, each refuse `invalid_runtime_input`/`wrong-role-mapping`; binding `k` to an object outside the universe refuses `invalid_runtime_input`/`invalid-value`. | Test (TC-650) |
| FR-205-AC-2 | The `Fill` subject has one initial state, whose key holds the root at `fork Both`, empty `observations`, `queues` and `bounds`, and `roles` binding `w`. After `fork(Both)` the key's `control` holds paths `(0)` at `join Both`, `(0, Both, left, 0)` at `A` running and `(0, Both, right, 0)` at `B` running; its maps are sorted by JCS bytes, and serializing the key twice gives equal bytes. Every reachable state of `Fill` has empty `observations`, because no node reads `a` or `b`; in a variant where a node after the join reads `a`, `observations` holds `a` in every state from `attempt(A)` to that node's step, under `(0, Both, left, 0)` until the join and under `(0)` after it, and in none after the node's step. In a variant where `left` is `sequence { attempt A; attempt A2 }`, `a` is held from `attempt(A)` to the join, and then by the root to the reading node. In a variant where `left` is `sequence { parallel R { branch x attempt X binding b } join all; attempt L2 }` and a node after `Both`'s join reads `b`, `x` keeps `b` through `join(R)`, `left` through `join(Both)`, and the root holds it until that node's step. | Test (TC-650) |
| FR-205-AC-3 | A `repeat` with maximum 2 around `parallel P { branch a attempt X …; branch b attempt Y …; } join any [a, b] outstanding continue`, where `X` and `Y` are always enabled: the second fork while instance 0's `b` is still `running` creates threads with ordinal 1; when `b` of instance 0 completes before the second fork, the second fork creates ordinal 0 again, so its threads' paths equal the first fork's. Two behaviours that reach the second fork's post-state with equal model state and counter, one through each completion order of the first instance's branches, reach one state. With `branch b sequence { parallel Q { branch x attempt X2; branch y attempt Y2 } join any [x, y] outstanding continue }` in place of `b`'s attempt, when instance 0's join has settled, `b` has completed and `y` of `Q` is still `running`, the second fork takes ordinal 1, because `y`'s path extends instance 0's branch path, and it takes ordinal 0 once `y` completes. With `maximum 3` and instances 0 and 1 both live, when instance 0 ends and is removed while instance 1 still runs, the third fork takes ordinal 0, the smallest ordinal no live sibling holds, and not 2. | Test (TC-650) |
| FR-205-AC-4 | A `Fill` subject with two initial snapshots (`c.v = 0` and `c.v = 1`) has two initial protocol states, each with the root at `fork Both`. An `on each` protocol's initial protocol state holds no instance. | Test (TC-650) |
| FR-205-AC-5 | A template `Refund` with two registrations, whose first activates and whose compensation thread `(0, Refund, 0)` is still live when the second activates: the second thread takes ordinal 1. When `cend` has removed `(0, Refund, 0)` before the second activates, the second thread takes ordinal 0. | Test (TC-650) |
| FR-205-AC-6 | Two protocol definitions in one package, each declaring `channel Pay`, give two subjects whose state keys hold `queues` entries, and whose `duplicate` and `lose` identities hold channel names, each in its own protocol's qualified name; no key or identity of one names the other's channel. | Test (TC-650) |
| FR-205-AC-7 | Inside a `repeat` of maximum 2: `parallel P { branch left attempt A1 binding a; branch right attempt B1 } join all`, then `parallel Q { branch u sequence { attempt U1; check Ck { a.ready } }; branch v attempt V1 } join any [u, v] outstanding continue`, where `A1` has two successors, binding `a` with `ready` true or false. After iteration 1's `join(P)` with `ready` true, the forking thread's binder environment holds `a` with `ready` true, and `fork(Q)` copies it into `u` and `v`. In the state where `V1` has been taken, the `join(Q)` step has run and `u` still rests at `U1`, the second fork of `P` takes ordinal 0, because `P`'s first instance is removed, while `Q`'s instance 0 is still live because `u` is `running`. In iteration 2 with `ready` false, `fork(Q)` takes ordinal 1 and gives its `u` an environment holding `a` with `ready` false: iteration 1's `u` still evaluates `Ck` over its own `a` (true) and iteration 2's `u` over its own (false, so it rests at `Ck`), and no state refuses. When `u` of iteration 1 completes, its entry is dropped, and the key's `observations` holds no `a` under its path. | Test (TC-650) |
| FR-205-AC-8 | Template `Refund` with `attempts 1` and `commit Committed`, whose forward `effect` node is reached twice in a `repeat` of maximum 2, giving registrations `r1` and `r2` that differ in a registration capture `id`, whose activation guard compares `id` with the `id` of the trigger record, and a `parallel Triggers` of two sibling branches each binding a trigger record with one of the two ids, followed by `commit Committed`, each compensation thread taking its `cattempt` and `cend` before the step of `Committed`. The state after `r1`'s trigger and then `r2`'s and the state after `r2`'s and then `r1`'s have different keys, because each compensation thread's entry names its registration: thread `(0, Refund, 0)` names `r1` in the first and `r2` in the second. The state after the step of `Committed` holds that node under its instance and the state before the step does not, so the two have different keys, and the entry is absent once the instance finishes. In a variant with `attempts 2 of M::RefundAttempt`, `RefundAttempt` with one field `ok: Boolean`, a template operation whose precondition is `true`, and the retry relation `not e.ok` over (`e`, `l`): after thread `(0, Refund, 0)`'s first `cattempt` with record `ok` false, `observations` holds that thread's attempt record with `ok` false and a retry `cattempt` is enabled; after its first `cattempt` with `ok` true, `observations` holds the attempt record with `ok` true and of the retry and end steps only `cend` and `cexpire` are enabled, so the two post-states have different keys. The attempt record is absent from `observations`, though kept in `control`'s attempt history, after the thread's second `cattempt`, whose count reaches 2, after its `cend`, and in the post-state of the step of `Committed` taken while the thread rests at its retry point holding one. With the retry relation `l.ok`, which does not read `e`, no state holds an attempt record in `observations` (PS-8 keeps one only for a retry relation that reads `earlier`), but every dispatched attempt is still recorded in `control` with its record, outcome and `retry_of` (FR-893), so the two first `cattempt` steps, `ok` true and `ok` false, reach two states with different keys, one per dispatched attempt; the two states have equal model state, equal attempt count 1, the equal per-attempt outcome `successful` and the equal enabled steps (one retry `cattempt` with `ok` true, its `cfail` and `ctimeout`, `cend` and `cexpire`), so they differ only in the historical attempt record. | Test (TC-650) |
| FR-205-AC-9 | The `Fill` subject's every state holds the `roles` entry `{"type":"model-binding","selection":"<M's identity>"}` for `w`; a `Fill` variant over a package selecting aliases `M` and `N`, with `role w on M` and `role u on N` and `u` owning an attempt on an `N` operation, builds with no binding for either role, and each state's `roles` entries name `M`'s and `N`'s identities (FSpec FR-425-AC-13, TC-375 PK-17 and PK-18). | Test (TC-650) |
| FR-205-AC-10 | Protocol `Echo` over the `Fill` model of AC-1 (`c.v = 0`, which no step of `Echo` writes): channel `ch` `fifo`, capacity 2, `delivery unknown`, carrying `S`; `run sequence { send Snd via ch; receive Rcv via ch of Snd }`; the claim `always holds(c.v = 0)`, which reads no delivery index. After `send`, the queue entry carries the message (instance 0, `Snd`, path `(0)`, ordinal 0) and delivery 0, and the message's `next-delivery` bound is 1 (state `s1`). From `s1`, `duplicate` then `lose` of delivery 0 reaches `s2`, whose queue holds delivery 1 and whose bound is 2; also from `s1`, `duplicate` then `lose` of the copy, delivery 1, reaches `s3`, whose queue holds delivery 0 and whose bound is 2. `s1`, `s2` and `s3` have three different `ProtocolKey`s and one cycle key, the `ProtocolKey` of `s1`. A second round from `s2` duplicates with delivery index 2 in its transition identity. EN-1 explores finitely many product states and settles the claim `Holds` (`proved`); a closure certificate holding the cycle keys is accepted. A cycle key that keeps the `next-delivery` bound or the raw delivery index of an unread channel fails this criterion. | Test (TC-650) |
| FR-205-AC-11 | Over `Echo` with the unread claim, `canon` is a bisimulation: from `s1` and from `s2`, the enabled steps pair one to one by kind and node (`duplicate`, `lose`, `receive`), each pair's post-states have equal cycle keys and equal model states, and the paired `receive` binds equal payloads; the paired `duplicate` steps take delivery 1 (from `s1`) and 2 (from `s2`), and their post-states have equal cycle keys. On an `unordered` variant of `ch` holding deliveries 1 and 3 of one message, `canon` renumbers them 0 and 1 and sets the bound to 2. | Test (TC-650) |
| FR-205-AC-12 | Read case. The delivery item over `Echo`: clause `on each` with `when (present(filament::protocol::delivery(Echo::Rcv)))`, capture `d` of that delivery, formula `not eventually[1, *] holds(filament::protocol::delivery(Echo::Rcv) = some(d))` under `filament.temporal.infinite-trace/v1` ("each delivery is received at most once", which has no counterexample). Its delivery and message read sets are `{Echo::ch}`, so `s1`, `s2` and `s3` have three different cycle keys, EN-1 with `max_depth` 8 settles `inconclusive` `BoundReached{depth: 8}`, and no run settles `proved`. The message item: the same clause with `filament::protocol::message(Echo::Rcv)` in place of the delivery read, capture `m`. Its message read set is `{Echo::ch}` and its delivery read set is empty, so `s1`, `s2` and `s3` keep one cycle key and the item settles `Holds`; over AC-15's `Loop`, the message item with `Loop::Rcv` keeps the ordinals, so the states for `k` = 0, 1 and 2 have three different cycle keys and EN-1 with `max_depth` 8 settles `inconclusive` `BoundReached{depth: 8}`. An item whose clause holds no step identity read has three empty read sets. A cycle key computed from read sets that ignore the clause's reads makes the delivery item `proved`, which this criterion rejects. | Test (TC-650) |
| FR-205-AC-13 | Counterexamples keep concrete labels. Re-concretising, from `Echo`'s initial state, the cycle-key path with prefix `send(Snd)`, `duplicate`, `lose` of the older copy and loop `duplicate`, `lose` of the older copy gives a counterexample whose `duplicate` identities carry delivery 1 and then 2, whose `lose` identities carry delivery 0 and then 1, and whose post-state keys are the concrete `ProtocolKey`s (`next-delivery` 2 after the first `duplicate`, 3 after the second); no delivery index appears in two `duplicate` identities. For the infinite-trace claim `eventually holds(c.v = 1)`, FR-217 replays it against the `scheduling adversarial` package with every step enabled and the loop closing on cycle keys, and settles `reproduced-with-evaluated-witness`. The same counterexample with the loop's `duplicate` relabelled to delivery 1 refuses `invalid_runtime_input`/`invalid-value` naming that step as not enabled. | Test (TC-650) |
| FR-205-AC-14 | Key members (FSpec FR-425-AC-15). `Echo`'s post-send queue entry is exactly `{message, delivery, payload}`, with no `send` or `thread` member. On a `fifo` channel of capacity 2 with delivery `[2,2]`, `send Snd`'s `count` 2 step queues deliveries 0 and 1 of one message (ordinal 0) and leaves `next-delivery` 2 and `message-ordinal` 1. With capacity 2, overflow `block` and delivery `[3,3]`, in `parallel P { branch s send Snd via ch; branch r receive Rcv via ch of Snd } join all`, the `count` 3 step queues deliveries 0 and 1 and leaves the send pending with count 1, `next-delivery` 2 and no `message-ordinal` for `Snd` on `s`'s path; after `receive`, the continuation step appends delivery 2 of the same message (ordinal 0), and only then is `next-delivery` 3 and `message-ordinal` 1. In `parallel P { branch a sequence { send Snd via ch; attempt A }; branch b receive Rcv via ch of Snd } join all`, `message-ordinal` 1 for `Snd` on `a`'s path is present while `a` rests at `A`, after `receive` and `attempt(A)`, when `a` has completed and no copy is queued, and in every later state until the instance is removed. Reused path: `repeat maximum 2 { parallel P { branch a send Snd via ch; branch b receive Rcv via ch of Snd } join all }` forks `a` on path `(0, P, a, 0)` in both iterations; the first message (ordinal 0) has been received when the second `send` runs, which takes ordinal 1, so the two `receive` steps bind deliveries of two different messages; a key that drops the ordinal when `a` completes or when its last copy leaves the queue gives both messages ordinal 0 and fails this criterion. | Test (TC-650) |
| FR-205-AC-15 | Unread message ordinals close a loop. Protocol `Loop` over the `Fill` model of AC-1: channel `ch` `fifo`, capacity 1, `delivery exactly-once-premise`; `run repeat { send Snd via ch; receive Rcv via ch of Snd }`, a `repeat` with no maximum; the claim `always holds(c.v = 0)`, which reads no message identity. After `k` iterations the state with the root back at `Snd` has `message-ordinal` `k` and `next-delivery` entries for the `k` received messages, so the states for `k` = 0, 1 and 2 have three different `ProtocolKey`s; `canon` removes the `next-delivery` entries of the received messages and the `message-ordinal` bound, so the three have one cycle key, the initial state's. The post-`send` states of iterations 1 and 2 hold messages with ordinals 0 and 1 and have one cycle key, whose entry names ordinal 0. EN-1 explores finitely many product states and settles the claim `Holds` (`proved`), and the lasso with an empty prefix and loop `send(Snd)`, `receive(Rcv)` closes on cycle keys under FR-217. A cycle key that keeps the ordinal or a received message's `next-delivery` fails this criterion. | Test (TC-650) |
| FR-205-AC-16 | Unread business-effect ordinals close a loop (FSpec FR-425-AC-16). Protocol `Charge` over the `Fill` model of AC-1: `run repeat { attempt Pay; effect Paid of Pay }`, a `repeat` with no maximum, `Pay` an operation that writes nothing and is always enabled; the claim `always holds(c.v = 0)`, which holds no step identity read. After `k` iterations the state with the root back at `Pay` has `business-effect-ordinal` `k` for `Paid` on `(0)` (none for `k` = 0), so the states for `k` = 0, 1 and 2 have three different `ProtocolKey`s; `canon` removes the entry, so the three have one cycle key, the initial state's. The `k`-th `event(Paid)` of a counterexample or replayed lasso carries the concrete effect occurrence key with ordinal `k - 1`, never relabelled. EN-1 explores finitely many product states and settles `Holds` (`proved`), and the lasso with an empty prefix and loop `attempt(Pay)`, `event(Paid)` closes on cycle keys under FR-217. A cycle key that keeps the ordinal never closes the loop and settles `BoundReached`, which this criterion rejects. | Test (TC-650) |
| FR-205-AC-17 | Read business-effect ordinals stay apart. Over AC-16's `Charge`, the effect item: clause `on each` with `when (present(filament::protocol::effect(Charge::Paid)))`, capture `e` of that occurrence, formula `not eventually[1, *] holds(filament::protocol::effect(Charge::Paid) = some(e))` under `filament.temporal.infinite-trace/v1` ("no `Paid` occurrence is established twice", which has no counterexample). Its effect read set is `{Charge::Paid}` and its message and delivery read sets are empty, so the states for `k` = 0, 1 and 2 keep their `business-effect-ordinal` entries and have three different cycle keys; EN-1 with `max_depth` 8 settles `inconclusive` `BoundReached{depth: 8}` and no run settles `proved`; the loop `attempt(Pay)`, `event(Paid)` offered as a lasso refuses under FR-217 and FR-329 as a loop that does not close. A variant `Charge3`, `Charge` with a second effect node declared after the `repeat` (`run sequence { repeat { attempt Pay; effect Paid of Pay }; attempt Close; effect Other of Close }`, `Close` never enabled), whose clause reads `filament::protocol::effect(Charge3::Other)` instead, keeps `Paid` outside the effect read set, so the loop states keep one cycle key and the item settles `Holds` (`proved`). | Test (TC-650) |


## Dependencies

- ADR-027 §1 PS-1 to PS-8 (PS-2 ordinals, PS-7 channel names), §2.1 FO-3, §2.2a AE-2.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (model subject), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`semantic` member and the state-key form),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-fspec-sampler.md)
  (state keys and coalescing), FR-218 (the checked protocol clause and
  scopes).
- FSpec owns the protocol subject and the content of the state key's
  members (ADR-027 QS-1, QS-3); FSL builds them. The message, delivery,
  `message-ordinal` and `next-delivery` members are FSpec FR-425-AC-15
  (agent-ix/filament-specification#742), and the `business-effect-ordinal`
  member FSpec FR-425-AC-16, on which this requirement depends.
- FSpec FR-828 owns the step identity reads whose occurrences in a clause
  form the read sets.
- [FR-894](FR-894-keep-message-delivery-and-business-effect-identities-apart.md)
  (delivery identity and what an obligation reads),
  [FR-217](FR-217-replay-a-protocol-counterexample.md) and
  [FR-329](FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md)
  (lasso closure by cycle key),
  [FR-338](FR-338-check-an-en-1-closure-certificate.md) (certificate states).

## References

- Owning ticket: Linear QSL-396. FSpec half: FSpec FR-425 (Linear STD-140).
