---
id: TC-3158
title: "Replay an every-scheduler expected total"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3158: Replay an every-scheduler expected total

## Description

Verify FR-202-AC-33: the no-random-line expected-total variation carries the
selected deterministic memoryless scheduler and a checked upper certificate.

## Test Procedure

Run the no-random-line model under every scheduler with claim
`expected total >= 1` and exact expected total 0. Replay the returned
counterexample against the same checked package and claim. Repeat with the
scheduler removed and with one scheduler choice changed.

## Expected Results

With the matching scheduler, the `upper` certificate is checked on its
induced chain, its initial bound is below 1, and the item settles `refuted`.
Removing or changing the scheduler yields no reproduced evidence and does not
settle `refuted`.
