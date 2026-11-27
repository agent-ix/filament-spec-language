---
id: TC-650
title: "The protocol subject builds, refuses bad bindings and keys states canonically"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: verifies
---
# TC-650: The protocol subject builds, refuses bad bindings and keys states canonically

## Description

Verify the protocol subject's construction and refusals, the thread paths and ordinals, binder removal and the canonical state key.

Scope: FR-205-AC-1 to FR-205-AC-18.

## Test Procedure

Use ADR-027 §7's `Fill` unit (universe `{c}`, `c.v = 0`).

1. Build the subject; build variants differing in `over` and in the
   `terminal` member and compare obligation identities; build with an absent
   protocol name, a binding supplied for `w`, an unbound `v` in the
   `role v on M::Cell` variant, and `k` bound outside the universe.
2. Read the initial key; take `fork(Both)` and read the key; serialize it
   twice; read `observations` in every reachable state; repeat with the
   variant whose post-join node reads `a`, the `sequence { attempt A;
   attempt A2 }` variant and the nested all-joined `parallel R` variant.
3. Run FR-205-AC-3's `repeat`/`join any … outstanding continue` protocol and
   read the paths created by each fork under both completion orders, and
   its `maximum 3` variant in which instance 0 ends while instance 1 runs,
   and its variant whose branch `b` holds an inner `parallel` with a
   running `y`.
4. Build `Fill` with two initial snapshots, and an `on each` protocol.
5. Activate FR-205-AC-5's two `Refund` registrations with and without
   `cend` between them.
6. Build subjects over two protocols that each declare `channel Pay`, and
   read a key with a queued entry and a `duplicate` identity from each.
7. Run FR-205-AC-7's `repeat` of `P` and `Q` with both values of `a.ready`;
   read the ordinals the second forks of `P` and `Q` take while the first
   `u` rests at `U1`, each `u`'s environment and `Ck` value, and the key's
   `observations`.
8. Run FR-205-AC-8's protocol: activate the two registrations by the two
   triggers in both orders, take each compensation thread's `cattempt` and
   `cend`, then step `Committed` and finish the instance;
   read the keys. Run its `attempts 2` variant with the retry relation
   `not e.ok`: take thread `(0, Refund, 0)`'s first `cattempt` with each
   record, then its retry `cattempt` and `cend`, and, on a separate path,
   step `Committed` while the thread rests at its retry point; read the
   `observations` member and the enabled steps of that thread after each
   step. Repeat with the retry relation `l.ok`.

9. Read `w`'s `roles` entry in every state of `Fill`; build FR-205-AC-9's
   two-alias variant with no role bindings and read both roles' entries.
10. Build FR-205-AC-10's `Echo` subject. Take `send(Snd)` to `s1`; from
    `s1` take `duplicate` then `lose` of delivery 0 to `s2`; separately,
    also from `s1`, take `duplicate` then `lose` of delivery 1 to `s3`;
    take a second `duplicate` from `s2`. Read each state's `ProtocolKey`
    and cycle key for the claim `always holds(c.v = 0)`. Run EN-1 on it
    with `max_depth` 8, and check a closure certificate of the cycle keys.
11. Pair the enabled steps at `s1` and `s2` and compare their post-states'
    cycle keys, model states and `receive` payloads; canonicalise an
    `unordered` variant holding deliveries 1 and 3 of one message.
12. FR-205-AC-12: repeat step 10's key reads and EN-1 run for the delivery
    item (clause holding `filament::protocol::delivery(Echo::Rcv)`) and the
    message item (clause holding `filament::protocol::message(Echo::Rcv)`
    only), read each item's three read sets, and run the message item with
    `Loop::Rcv` over FR-205-AC-15's `Loop` for 0, 1 and 2 iterations.
13. Re-concretise from `Echo`'s initial state the cycle-key path with
    prefix `send`, `duplicate`, `lose` of the older copy and loop
    `duplicate`, `lose` of the older copy; replay it through FR-217 for
    `eventually holds(c.v = 1)` against the `scheduling adversarial`
    package; replay it again with the loop's `duplicate` relabelled to
    delivery 1.
14. Read `Echo`'s post-send queue entry. On a `fifo` channel of capacity 2
    with delivery `[2,2]`, take `send Snd`'s `count` 2 step. With capacity
    2, overflow `block`, delivery `[3,3]`, in `parallel P { branch s send
    Snd via ch; branch r receive Rcv via ch of Snd } join all`, take the
    `count` 3 step, `receive`, then the continuation step. In `parallel P {
    branch a sequence { send Snd via ch; attempt A }; branch b receive Rcv
    via ch of Snd } join all`, take `fork`, `send`, `receive`, `attempt(A)`,
    the join and `finish`. Read `queues` and `bounds` after each step.
15. Reused branch path: run `repeat maximum 2 { parallel P { branch a send
    Snd via ch; branch b receive Rcv via ch of Snd } join all }` through
    both iterations; read each `send`'s message identity and each
    `receive`'s delivery identity.
16. Build FR-205-AC-15's `Loop` subject. Read the `ProtocolKey` and cycle
    key of the state with the root at `Snd` after 0, 1 and 2 iterations,
    and of the post-`send` states of iterations 1 and 2. Run EN-1 on
    `always holds(c.v = 0)`; replay the lasso with an empty prefix and loop
    `send(Snd)`, `receive(Rcv)` through FR-217.
17. FR-205-AC-16: build `Charge` (`run repeat { attempt Pay; effect Paid of
    Pay }`). Read the `ProtocolKey` and cycle key of the state with the root
    at `Pay` after 0, 1 and 2 iterations; run EN-1 on `always holds(c.v =
    0)`; replay the lasso with an empty prefix and loop `attempt(Pay)`,
    `event(Paid)` through FR-217 and read its `event` identities.
18. FR-205-AC-17: repeat step 17's key reads and EN-1 run (`max_depth` 8)
    for the effect item holding `filament::protocol::effect(Charge::Paid)`,
    and offer step 17's lasso for it; then run FR-205-AC-17's `Charge3` variant, whose clause reads
    `effect(Charge3::Other)`.
19. Build `Fill` over the universe `[c, d]`, only `c` held by the initial
    state, with `k` bound to `c` and to `d`; then with an empty initial list
    and `k` bound to `d`.

Tag the tests `#[trace("TC-650", "FR-205-AC-n")]`.

## Expected Results

- Step 1: the subject builds; identities differ; the three refusals of
  FR-205-AC-1, each naming its protocol, role or parameter.
- Step 2: the keys of FR-205-AC-2, sorted by JCS bytes and byte-equal on
  re-serialization; `observations` empty throughout `Fill`; in the variant
  the binder is present exactly from `attempt(A)` to the reading step, in
  the multi-step variant through the join, and in the nested variant `b` is
  held by `x`, then `left`, then the root.
- Step 3: ordinal 1 while instance 0's `b` runs, ordinal 0 otherwise; the
  two histories meet in one state; with `maximum 3`, the third fork takes
  ordinal 0 once instance 0 is removed while instance 1 runs; ordinal 1
  while `y` runs and 0 after it completes.
- Step 4: two initial states at `fork Both`; no instance in the `on each`
  initial state.
- Step 5: ordinal 1 with `(0, Refund, 0)` live, ordinal 0 after `cend`.
- Step 6: each queue key and identity names its own protocol's qualified
  channel name.
- Step 7: `P` forks at ordinal 0 again and `Q` at ordinal 1; each `u` reads its
  own iteration's `a` (true, then false) and no state refuses; `observations`
  holds no `a` under a completed `u`.
- Step 8: distinct keys for the two activation orders, each thread's entry
  naming its registration; the commit entry from the commit step until the
  instance finishes. In the `not e.ok` variant, the thread's attempt record
  with the first attempt's `ok` value after its first `cattempt`, a retry
  `cattempt` enabled after `ok` false and only `cend` and `cexpire` after `ok` true,
  distinct keys for the two records, and no attempt record after the second
  `cattempt`, after `cend` or after the step of `Committed`. In the `l.ok`
  variant, no attempt record in `observations` in any state, and two states
  with different keys after the two first `cattempt` steps, equal in model
  state, count, outcome `successful` and enabled steps.
- Step 9: `w`'s entry is `{"type":"model-binding","selection":"<M's
  identity>"}` in every state; the two-alias variant builds, with entries
  naming `M`'s and `N`'s identities (FR-205-AC-9).
- Step 10: three different `ProtocolKey`s and, for the unread claim, one
  cycle key (`s1`'s `ProtocolKey`); the second `duplicate` from `s2` carries
  delivery index 2; EN-1 settles `Holds` (`proved`) and the certificate is
  accepted. The step fails if the cycle key keeps the `next-delivery` bound
  or the raw delivery index of an unread channel.
- Step 11: the steps pair one to one (`duplicate`, `lose`, `receive`) with
  equal cycle keys, model states and payloads, the paired `duplicate` steps
  taking deliveries 1 and 2; the `unordered` copies become deliveries 0 and
  1 with bound 2.
- Step 12: the delivery item's delivery and message read sets are
  `{Echo::ch}`; the message item's message read set is `{Echo::ch}` and its
  delivery read set is empty. For the delivery item, three different cycle
  keys and `inconclusive` `BoundReached{depth: 8}`, never `proved`; for the
  message item, one cycle key and `Holds` over `Echo`, and over `Loop` three
  cycle keys and `BoundReached{depth: 8}`.
- Step 13: `duplicate` identities carry deliveries 1 then 2, `lose`
  identities 0 then 1, post-state keys are concrete (`next-delivery` 2,
  then 3), and no index repeats; the replay settles
  `reproduced-with-evaluated-witness`. The relabelled replay refuses
  `invalid_runtime_input`/`invalid-value` naming the loop's `duplicate` as
  not enabled.
- Step 14: the `Echo` entry is exactly `{message, delivery, payload}`. The
  `[2,2]` step queues deliveries 0 and 1 of one message (ordinal 0), with
  `next-delivery` 2 and `message-ordinal` 1. The `[3,3]` step queues
  deliveries 0 and 1, leaves count 1 pending, `next-delivery` 2 and no
  `message-ordinal` for `Snd`; the continuation appends delivery 2 of the
  same message (ordinal 0), then `next-delivery` 3 and `message-ordinal` 1.
  `message-ordinal` 1 for `Snd` on `a`'s path in every state from the
  `send` until the instance is removed, including after `a` has completed
  with no copy queued. The step fails if the continuation takes ordinal 1,
  an index is reused, or the ordinal is dropped before the instance is
  removed.
- Step 15: both iterations' `a` has path `(0, P, a, 0)`; the first message
  (ordinal 0) has been received when the second `send` runs, which takes
  ordinal 1; the two `receive` steps bind deliveries of two different
  messages. The step fails if both messages have ordinal 0.
- Step 16: three different `ProtocolKey`s (`message-ordinal` 0 absent, 1, 2)
  and one cycle key, the initial state's; the two post-`send` states have
  one cycle key naming ordinal 0; EN-1 settles `Holds` (`proved`); the
  replay closes the loop on cycle keys. The step fails if the cycle key
  keeps the ordinal or a received message's `next-delivery`.
- Step 17: three different `ProtocolKey`s (`business-effect-ordinal`
  absent, 1, 2) and one cycle key, the initial state's; EN-1 settles `Holds`
  (`proved`); the replay closes on cycle keys and its `event(Paid)`
  identities carry ordinals 0, 1, … in order, never relabelled. The step
  fails if the cycle key keeps the ordinal.
- Step 18: the effect item's effect read set is `{Charge::Paid}` and its
  message and delivery read sets are empty; three different cycle keys;
  `inconclusive` `BoundReached{depth: 8}`, never `proved`; the lasso refuses
  `invalid_runtime_input`/`invalid-value` as a loop that does not close. The
  `Charge3` variant settles `Holds` (`proved`).
- Step 19: builds; refuses `invalid_runtime_input`/`invalid-value` naming `k` and `d`; not refused, and the item settles `inconclusive` `NoInitialState`.
