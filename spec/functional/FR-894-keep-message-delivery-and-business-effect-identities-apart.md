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
| FR-894-AC-1 | Over a channel `delivery at-least-once` carrying `Charge` from `Buyer` to `Seller`, whose receive is followed by `attempt Pay` and `effect Paid of Pay`, a behaviour that duplicates the send has two deliveries (indices 0 and 1) of one message identity. At most one receive takes a delivery, and exactly one `Paid` effect occurrence is established. A claim "`Paid` occurs at most once per instance" is `proved`. | Test (TC-2528) |
| FR-894-AC-2 | A recorded execution (FR-895) in which two business-effect keys are bound to one `Paid` occurrence key reports one `duplicate-business-effect` fact naming both keys. The registration's attempt and recovery facts are unchanged beside it. | Test (TC-2528) |
| FR-894-AC-3 | A recording whose first attempt is `failed` with no effect, and whose retry is `successful` with one effect, reports no duplicate. A recording in which both attempts are `successful`, each with its own effect key for the one occurrence, reports `duplicate-business-effect`. | Test (TC-2528) |
| FR-894-AC-4 | Two effects with different occurrence keys (two `repeat` iterations, or two instances) are never reported as duplicates, whatever their payloads. One observation repeated with equal facts reports `replayed-observation`, counts once and creates no fact. | Test (TC-2528) |
| FR-894-AC-5 | A recording with two receive observations, of deliveries 0 and 1 of one message, reports `duplicate-delivery`, on a channel whose delivery policy admits duplication. The same recording on an `exactly-once-premise` channel reports a violated delivery premise (FR-895). Each report's record names the duplicate kind. | Test (TC-2529) |

## Dependencies

- ADR-027 ST-3 to ST-5, TS-2: send records, copies and transition identity.
- [FR-206](FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md):
  the step kinds.
- [FR-893](FR-893-reach-and-report-every-compensation-outcome.md): outcome
  facts.
- [FR-895](FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md):
  recorded executions.
- FSpec FR-051, FR-058, FR-224, FR-226 and FR-236.
