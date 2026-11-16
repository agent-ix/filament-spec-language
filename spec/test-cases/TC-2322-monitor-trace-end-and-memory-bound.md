---
id: TC-2322
title: "Finishing adds no closure, and retained positions bound the values held"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-2322: Finishing adds no closure, and retained positions bound the values held

## Description

Verify that `finish()` returns the trace's own reading, and that
`retained_positions` bounds the values a session holds over a long stream
while a window clause still decides correctly. Scope: FR-643-AC-8,
FR-643-AC-9.

## Test Procedure

1. Finish a `Finite` session over FR-643-AC-1's snapshots; finish a `Progressed` session whose latest assertion has scope `open` and execution `closed`.
2. Run a `Finite` session over 1,000 samples cycling 0 to 8 with `retained_positions` 4 for `Below`, `Nine` and `Recent`, reading the values `Recent`'s state holds after each step; checkpoint after the last sample.
3. Run `Recent` over levels 2, 0, 2.

## Expected Results

- Step 1: the `Finite` outcome equals `monitor` over the trace; the `Progressed` outcome is `B(n)` under scope `open` and execution `closed`.
- Step 2: `Below` and `Nine` stay `Pending` with no `incomplete`; `Recent` holds `min(k, 4)` values after step `k` and settles `success` at step 10; the checkpoint holds no admitted value.
- Step 3: `Recent` settles `violation` at step 3.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`.
