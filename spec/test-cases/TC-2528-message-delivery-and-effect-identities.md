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

Verify that messages, deliveries and business effects keep separate identities. Scope: FR-894-AC-1, FR-894-AC-2, FR-894-AC-3, FR-894-AC-4.

Catches identity derived from payload equality or from a transport fact.

## Test Procedure

1. Explore the `at-least-once` `Charge` protocol with a duplicated send; assess recordings with two effect keys for one occurrence, a failed-then-successful retry, two successful attempts, two iterations and a repeated observation.

## Expected Results

- Step 1: The duplicate gives two deliveries of one message and one `Paid` effect, and the at-most-once claim is `proved`; the recordings report `duplicate-business-effect`, nothing, `duplicate-business-effect`, nothing and `replayed-observation` respectively.
