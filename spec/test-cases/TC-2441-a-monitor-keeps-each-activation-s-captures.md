---
id: TC-2441
title: "A monitor and a carried clause state keep each activation's captures across steps and restores"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-860
    type: verifies
---
# TC-2441: A monitor and a carried clause state keep each activation's captures across steps and restores

## Description

Verify that the stepping monitor and FR-820's carried clause state fix a
capture once at its activation, report after each step what the batch run
over the prefix reports, and keep the capture across a serialized restore
without evaluating it again. Scope: FR-860-AC-5, FR-860-AC-6.

## Test Procedure

1. Stream `Fast` over records levels 5, 1, 1, 1 (`stopped` false) through a `native-monitor/1` request, and run `Fast` through `native-run/1` over each prefix.
2. Advance a `ClauseState` of `Fast` over positions 0 and 1, write it with `to_bytes`, read it with `from_bytes`, advance it over positions 2 and 3, and compare with the uninterrupted state.

## Expected Results

- Step 1: unsettled at records 0 to 2; settled `violation` at record 3 with settlement step `4`, trigger `decided`; each disposition equals the batch run over the prefix.
- Step 2: equal reports, `violation` at position 0 after position 3; the capture is not evaluated again and is not counted in `retained()`.

Test: `tests/it/temporal_capture_run.rs` (step 1) and `fsl-analyze/tests/it/monitor_incremental.rs` (step 2, with FR-820's `ClauseState`).
