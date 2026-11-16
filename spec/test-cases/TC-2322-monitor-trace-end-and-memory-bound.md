---
id: TC-2322
title: "The end record closes the stream only, and retained positions bound memory"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-2322: The end record closes the stream only, and retained positions bound memory

## Description

Verify that `finish(Closed)` and `finish(Open)` agree for every kind, and that
`retained_positions` bounds the values a session holds over a long stream.
Scope: FR-643-AC-8, FR-643-AC-9.

## Test Procedure

1. Finish a `Finite` session over FR-643-AC-1's snapshots with `Closed` and with `Open`; finish a `Progressed` session whose last assertion has scope `open` both ways.
2. Run a `Finite` session over 1,000 samples cycling 0 to 8 with `retained_positions` 4 for `Below`, `Nine` and `Recent`; checkpoint after the last sample.

## Expected Results

- Step 1: equal outcomes in each pair; the progressed one is the open-scope outcome.
- Step 2: `Below` and `Nine` stay `Pending` with no `incomplete`; `Recent` settles `success` at step 10; the checkpoint holds no admitted value.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`.
