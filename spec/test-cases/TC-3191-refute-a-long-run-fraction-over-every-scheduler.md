---
id: TC-3191
title: "Refute a long-run fraction over every scheduler"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3191: Refute a long-run fraction over every scheduler

## Description

Verify FR-202-AC-45: a false every-scheduler long-run claim is refuted only
by replaying the witness scheduler's induced chain with its long-run
subsystem certificate.

## Test Procedure

Over `Avail2`, run `long-run fraction holds(up) >= rational(9993, 10000)`
and `<= rational(999, 1000)` over every scheduler. Replay each emitted
counterexample, then replay it with the scheduler swapped, removed or
randomized, at the equality threshold, and with a proof document in place
of the subsystem certificate.

## Expected Results

The `repair_slow` witness refutes the AtLeast claim and the `repair_fast`
witness refutes the AtMost claim, each with matching reproduced evidence.
Every altered variant produces no reproduced token.
