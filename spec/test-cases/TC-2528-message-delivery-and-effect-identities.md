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

Verify that messages, deliveries and business effects keep separate identities, and that obligations read them through step identity reads. Scope: FR-894-AC-1, FR-894-AC-2, FR-894-AC-3, FR-894-AC-4, FR-894-AC-6 to FR-894-AC-9.

Catches identity derived from payload equality or from a transport fact.

## Test Procedure

1. Explore the `at-least-once` `Charge` protocol with a duplicated send; assess recordings with two effect keys for one occurrence, a failed-then-successful retry, two successful attempts, two iterations and a repeated observation.
2. Check and model-check FR-894-AC-6's delivery obligation over `Charge`, and the delivery and message obligations over `Charge2`.
3. Model-check FR-894-AC-7's `Paid` obligation over `ChargeLoop` with `max_depth` 8, assess it over the per-iteration recording and the two-key recording, check `filament::protocol::delivery(ChargeLoop::Snd)`, and request the obligation over a model subject.
4. Model-check FR-894-AC-8's obligation over `Once` with two sequential activations, and assess it over a recording of the two runs.
5. Check FR-894-AC-9's `requires temporal` cases and the capture and model-field types.

## Expected Results

- Step 1: The duplicate gives two deliveries of one message and one `Paid` effect, and the at-most-once claim is `proved`; the recordings report `duplicate-business-effect`, nothing, `duplicate-business-effect`, nothing and `replayed-observation` respectively.
- Step 2: the delivery obligation checks with one `protocol`/`step_read` node and has no counterexample; over `Charge2` the delivery obligation has none and the message obligation is refuted with one message read at deliveries 0 and 1. The step fails if a read compares payloads.
- Step 3: `inconclusive` `BoundReached{depth: 8}` in exploration; `satisfied` over the per-iteration recording; over the two-key recording `violated` at the second `Paid` observation, the item `refuted` there, beside the `duplicate-business-effect` fact; the `delivery(ChargeLoop::Snd)` read refuses `ill_typed`/`operator-ineligible`, and the model-subject request `invalid_runtime_input`/`invalid-value` naming the read.
- Step 4: not refuted in exploration nor over the recording.
- Step 5: `definition-cycle` at `Charge`'s requirement; `Drain` and the request over `Charge` admitted; the capture checks and the model field refuses `ill_typed`/`operator-ineligible`.
