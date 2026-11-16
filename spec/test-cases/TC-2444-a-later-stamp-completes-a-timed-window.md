---
id: TC-2444
title: "A later stamp completes a timed window with no through"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: verifies
---
# TC-2444: A later stamp completes a timed window with no through

## Description

Verify that a finite timed trace with no `through` reads a window as
complete once a later stamp passes its end, so a violation inside it is
reported, while a window no stamp has passed stays open. Scope:
FR-642-AC-6.

## Test Procedure

1. Evaluate `Ack` over the finite timed trace `req` at 0, `ack` at 250, with no `through`.
2. Evaluate `Ack` over `req` at 0, `ack` at 200, with no `through`.
3. Evaluate `Ack` over `req` at 0 alone, with no `through`.

## Expected Results

- Step 1: `violation` at position 0.
- Step 2: `pending`.
- Step 3: `pending`.

Test: `fsl-eval/tests/it/timed_recorded_trace.rs`.
