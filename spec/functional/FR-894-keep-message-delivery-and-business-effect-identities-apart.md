---
id: FR-894
title: "Keep message, delivery and business-effect identities apart"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-893
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-051
    type: implements
  - target: ix://agent-ix/filament-specification/FR-058
    type: implements
  - target: ix://agent-ix/filament-specification/FR-224
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-226
    type: implements
  - target: ix://agent-ix/filament-specification/FR-236
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-828
    type: implements
---
# FR-894: Keep message, delivery and business-effect identities apart

## Description

FSL SHALL give a protocol's messages, deliveries and business effects three
separate identities, in the protocol system and in the assessment of a
recorded execution (FR-895). It SHALL never derive one identity from
another (FSpec FR-051, FR-226).

- **Message identity.** A `send` step's send record (ADR-027 ST-3) is the
  message identity: the protocol instance, the send node, the thread path and
  the send's occurrence ordinal on that path.
- **Delivery identity.** A message has one delivery for each copy in its
  queue. The original copy is delivery 0, and each `duplicate` step (ST-5)
  adds the next delivery index to the same message identity. A `receive` step
  binds one delivery: its transition identity carries the message identity
  and the delivery index.
- **Business-effect identity.** An `effect` step establishes one business
  effect of one attempt occurrence. Its occurrence key is the protocol
  instance, the effect node, the thread path and the occurrence ordinal of
  that node on the path, which advances once per `repeat` iteration. In a
  recorded execution, the effect's key is the observation's FSpec FR-287
  `business-effect` reference, and the occurrence it is bound to is the same
  occurrence key.

These identities give four rules:

1. **A redelivery is not a second effect.** Receiving a second delivery of a
   message binds a second delivery of the same message identity. It
   establishes no effect, because an effect is established only by an
   `effect` step (FSpec FR-226-AC-2). A receive node on one thread path is
   taken once per occurrence, so a duplicate copy left in the queue is
   received only by a later receive node or not at all.
2. **Two keys for one occurrence are a duplicate business effect.** In a
   recorded execution, two different business-effect keys bound to one
   effect occurrence key make one duplicate-business-effect fact `duplicate`
   (FSpec FR-226-AC-3, FR-236). The fact is keyed by the registration, where
   the effect registers one, and by the second effect key. It sits beside
   the attempt and recovery facts and relabels neither.
3. **A retry is not a duplicate.** An effect established by a retried
   attempt, whose predecessor failed or timed out and established no effect,
   is the occurrence's one effect. Two successful attempts of one
   registration that each establish an effect for one occurrence key make a
   duplicate business effect.
4. **Replay is not a duplicate.** Re-observing one complete key with equal
   binding facts is replay of one subject. It counts once and creates no fact
   (FSpec FR-058).

In exploration, an effect occurrence key is established at most once, by
construction, so no `duplicate` fact arises (FR-893). A protocol obligation
may read the message identity, the delivery index and the effect occurrence
key of the steps it observes, so a claim can state "each message is
received at most once" or "each charge is applied once" over every behaviour.
A delivery index that the checked item reads stays in the state key and
keeps every protocol state apart that it tells apart. One that the item does
not read is canonicalised in the cycle key that coalesces states and closes
lassos (FR-205, "Cycle key"), so a `duplicate` and `lose` loop closes there
while its counterexample still carries the concrete, never-reused indices.

### Step identity reads

An obligation reads these identities through FSpec FR-828's step identity
reads, `filament::protocol::message(N)`, `filament::protocol::delivery(N)` and
`filament::protocol::effect(N)`. FSL SHALL check, carry and evaluate them as
FSpec FR-828 states:

- **S3.** The checker resolves the read's protocol and node, types it
  `Option<filament::protocol::Message>`, `Option<filament::protocol::Delivery>`
  or `Option<filament::protocol::EffectOccurrence>`, refuses each form FSpec
  FR-828 refuses with its kind and location (an identity type outside a
  capture's type included), and emits one FSpec FR-322 `protocol`/`step_read`
  node per (field, node) pair, reached from the temporal clause's formula,
  captures or activation guard. A protocol whose `requires temporal` names a
  clause holding a read of that protocol's own node refuses
  `invalid_package`/`definition-cycle` at the requirement, naming the
  protocol and the clause, as FR-218-AC-25 refuses a requirement whose
  clause names its protocol. S3 also records, per temporal clause, the
  channels and effect nodes its reads name, from which FR-205 derives an
  item's read sets.
- **Evaluation.** At a position of a protocol behaviour the evaluator takes
  the read's value from the transition identity of the position's step
  (ADR-027 TS-2, FSpec FR-426): the `message` member of a `send` or
  `receive` step of `N`, the `message` and `delivery` members of a `receive`
  step of `N`, the `effect_occurrence` member of an `event` step of `N`;
  every other position gives `none`. Because the members name the entry a
  `receive` removes, two `receive` successors that bind different copies have
  different identities and different read values. Replay (FR-217) and a
  supplied trace (FR-636 `finite-replay`) evaluate the same reads from the
  identities their steps carry.
- **Instance scope.** An identity is unique only within one protocol
  instance's lifetime, because a later instance can reuse a finished
  instance's ordinal (FSpec FR-425). Each obligation instance (one per
  activation, FR-860) is bound to exactly one protocol instance lifetime, as
  FSpec FR-828 "Instance scope of an obligation" states: inside it a read is
  `none` at a position whose step belongs to another protocol instance and
  at every position after the bound instance is removed. Captures belong to
  their obligation instance and are never compared across obligation
  instances, so a captured identity cannot equal a byte-equal identity of a
  later lifetime. EN-1's product state holds, per live obligation instance,
  whether its bound instance has been removed.
- **Recordings.** FR-895's replay mode reads a `requires temporal` clause as
  its trace subject; by the S3 rule such a clause holds no read of its own
  protocol. A temporal clause of the package that holds reads of the
  recorded protocol is assessed there as an identity-claim obligation (FR-895):
  each observation is a position, a read takes the identities the
  observation's FSpec FR-051 bindings name, and protocol instances are told
  apart by their FSpec FR-287 keys, so exploration, replay and assessment
  read the same values for the same steps.
- **Visibility.** A step is visible to a claim (FR-216) when a step
  identity read of the claim names its node, because the read's value
  changes at that step's position.
- **Subject.** The pre-check refuses an item whose subject is not a protocol
  subject of the protocol a read names `invalid_runtime_input`/
  `invalid-value`, naming the read, before any state is explored.

FSL SHALL name, in the protocol-analysis record (FR-574) and the replay-mode
report (FR-895), the kind of each duplicate found, as one of three values:
`duplicate-delivery`, `duplicate-business-effect` or `replayed-observation`.

### Decision: identity comes from the occurrence, never from the payload

A redelivered message and a second charge can carry equal payloads, and a
first charge and a retried charge can carry different ones. Only the
occurrence tells them apart: which send, which copy, which effect node on
which path in which instance. FSpec FR-051 forbids inferring an effect from a
transport fact or an equal payload, and FR-226 keys an effect by its
authored identity. FSL keys each subject by its occurrence, and so one
evaluator serves exploration and recorded runs. No consumer has to rebuild
the distinction in a ledger of its own.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-894-AC-1 | Fixture `Charge`: channel `ch` `fifo`, capacity 2, `delivery at-least-once`, carrying `Charge` from `Buyer` to `Seller`; `run sequence { send Snd via ch; receive Rcv via ch of Snd; attempt Pay; effect Paid of Pay }`. A behaviour that duplicates the send has two deliveries (indices 0 and 1) of one message identity. At most one receive takes a delivery, and exactly one `Paid` effect occurrence is established. A claim "`Paid` occurs at most once per instance" is `proved`. | Test (TC-2528) |
| FR-894-AC-2 | A recorded execution (FR-895) in which two business-effect keys are bound to one `Paid` occurrence key reports one `duplicate-business-effect` fact naming both keys. The registration's attempt and recovery facts are unchanged beside it. | Test (TC-2528) |
| FR-894-AC-3 | A recording whose first attempt is `failed` with no effect, and whose retry is `successful` with one effect, reports no duplicate. A recording in which both attempts are `successful`, each with its own effect key for the one occurrence, reports `duplicate-business-effect`. | Test (TC-2528) |
| FR-894-AC-4 | Two effects with different occurrence keys (two `repeat` iterations, or two instances) are never reported as duplicates, whatever their payloads. One observation repeated with equal facts reports `replayed-observation`, counts once and creates no fact. | Test (TC-2528) |
| FR-894-AC-5 | A recording with two receive observations, of deliveries 0 and 1 of one message, reports `duplicate-delivery`, on a channel whose delivery policy admits duplication. The same recording on an `exactly-once-premise` channel reports a violated delivery premise (FR-895). Each report's record names the duplicate kind. | Test (TC-2529) |
| FR-894-AC-6 | Over AC-1's `Charge`, under `filament.temporal.infinite-trace/v1`, the obligation "each delivery is received at most once" (`on each` with `when (present(filament::protocol::delivery(Charge::Rcv)))`, capture `d: filament::protocol::Delivery = value(filament::protocol::delivery(Charge::Rcv))`, formula `not eventually[1, *] holds(filament::protocol::delivery(Charge::Rcv) = some(d))`) checks, its package holds one `protocol`/`step_read` node for (`delivery`, `Rcv`), and it has no counterexample. Variant `Charge2`, AC-1's fixture with `receive Rcv2 via ch of Snd` after `Rcv`: the obligation over both receives (guard `present(filament::protocol::delivery(Charge2::Rcv)) or present(filament::protocol::delivery(Charge2::Rcv2))`, capture of whichever is present, formula `not eventually[1, *] (holds(filament::protocol::delivery(Charge2::Rcv) = some(d)) or holds(filament::protocol::delivery(Charge2::Rcv2) = some(d)))`) has no counterexample, while the same obligation over `filament::protocol::message` is refuted by the behaviour that duplicates the send and receives both copies, whose two receive positions read one message with deliveries 0 and 1. A read evaluated from the payload makes the two deliveries equal and refutes the delivery obligation, which this criterion rejects. | Test (TC-2528) |
| FR-894-AC-7 | Variant `ChargeLoop`: AC-1's fixture with `run repeat { send Snd via ch; receive Rcv via ch of Snd; attempt Pay; effect Paid of Pay }`, a `repeat` with no maximum. "No `Paid` occurrence is established twice" (`on each` on `present(filament::protocol::effect(ChargeLoop::Paid))`, capture `e`, `not eventually[1, *] holds(filament::protocol::effect(ChargeLoop::Paid) = some(e))`) has no counterexample within `max_depth` 8 and settles `inconclusive` `BoundReached{depth: 8}` in exploration (FR-205-AC-17). Assessed as an identity-claim obligation over a recording with one `Paid` occurrence per iteration it is `satisfied`; over a recording whose second iteration's `Paid` observation is bound to the first iteration's occurrence key under a second business-effect key, both observations read that one occurrence key, so the obligation is `violated` at the second `Paid` observation and the item settles `refuted` there, beside AC-2's `duplicate-business-effect` fact naming both keys. `filament::protocol::delivery(ChargeLoop::Snd)` refuses `ill_typed`/`operator-ineligible` at S3, and the obligation requested over a model subject refuses `invalid_runtime_input`/`invalid-value` naming the read. | Test (TC-2528) |
| FR-894-AC-8 | Instance scope. Protocol `Once`, `activation on each`, whose instance runs `sequence { send Snd via ch; receive Rcv via ch of Snd }` and finishes, activated twice in sequence, so the second instance reuses ordinal 0 and its message is byte-equal to the first's. The obligation `on each` on `present(filament::protocol::message(Once::Rcv))`, capture `m`, `not eventually[1, *] holds(filament::protocol::message(Once::Rcv) = some(m))` is not refuted in exploration: the obligation instance activated at the first receive reads `none` at the second instance's receive, because its bound instance has been removed. It is not refuted over a recording of the two runs either. A product that compares raw identities across the two lifetimes refutes it, which this criterion rejects. | Test (TC-2528) |
| FR-894-AC-9 | `Charge` with `requires temporal Due;`, where `Due` holds `filament::protocol::delivery(Charge::Rcv)`, refuses `invalid_package`/`definition-cycle` at the requirement, naming `Charge` and `Due`, with no package; the same `Due` required by a protocol `Drain` that it does not read checks, and `Due` requested over `Charge`'s protocol subject is admitted. A capture typed `filament::protocol::Message` checks, and a model field typed `filament::protocol::Message` refuses `ill_typed`/`operator-ineligible` at the type. | Test (TC-2528) |

## Dependencies

- ADR-027 ST-3 to ST-5, TS-2: send records, copies and transition identity.
- [FR-206](FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md):
  the step kinds.
- [FR-205](FR-205-define-the-protocol-subject-and-its-state-key.md): the
  state key and the cycle key.
- [FR-893](FR-893-reach-and-report-every-compensation-outcome.md): outcome
  facts.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  recorded executions.
- FSpec FR-051, FR-058, FR-224, FR-226 and FR-236.
