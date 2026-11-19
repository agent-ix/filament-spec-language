---
id: TC-2528
title: "Messages, deliveries and business effects keep separate identities"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-894
    type: verifies
---
# TC-2528: Messages, deliveries and business effects keep separate identities

## Description

Verify that messages, deliveries and business effects keep separate identities, and that obligations read them through step identity reads. Scope: FR-894-AC-1, FR-894-AC-2, FR-894-AC-3, FR-894-AC-4, FR-894-AC-6, FR-894-AC-7.

Catches identity derived from payload equality or from a transport fact.

## Test Procedure

1. Explore the `at-least-once` `Charge` protocol with a duplicated send; assess recordings with two effect keys for one occurrence, a failed-then-successful retry, two successful attempts, two iterations and a repeated observation.
2. Check and model-check FR-894-AC-6's delivery obligation over `Charge`, and the message obligation over the two-receive variant.
3. Model-check FR-894-AC-7's `Paid` obligation, assess it over the per-iteration recording and the two-key recording, check `filament::protocol::delivery(Charge::Snd)`, and request the obligation over a model subject.

## Expected Results

- Step 1: The duplicate gives two deliveries of one message and one `Paid` effect, and the at-most-once claim is `proved`; the recordings report `duplicate-business-effect`, nothing, `duplicate-business-effect`, nothing and `replayed-observation` respectively.
- Step 2: the delivery obligation checks with one `protocol`/`step_read` node and holds; the two-receive message variant is refuted with one message read at deliveries 0 and 1. The step fails if a read compares payloads.
- Step 3: the `Paid` obligation holds in exploration and over both recordings, and the two-key recording still reports only `duplicate-business-effect`; the `delivery(Charge::Snd)` read refuses `ill_typed`/`operator-ineligible`, and the model-subject request `invalid_runtime_input`/`invalid-value` naming the read.
