---
id: TC-1547
title: "The late-data cutoff in the finite-window evaluator"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: verifies
---
# TC-1547: The late-data cutoff in the finite-window evaluator

## Description

Verify late-data classification, delayed establishment, the `late` list and the independence of late-data and history cutoffs.

Scope: FR-501-AC-29, FR-501-AC-30.

## Test Procedure

1. Under `AllowedLateness { lateness: 2 }`, evaluate AC-29's clause at `W` 6 and 7, with the stamp-4 observation true, then with only the stamp-3 observation true.
2. Under `FixedInstant { cutoff: 5 }`, classify observations stamped 3 and 6 arriving at 6.
3. Evaluate with no cutoff, then with `HistoryCutoff` and the cutoff together under `once[0,4] holds(p)`.

Tag the tests `#[trace("TC-1547", "FR-501-AC-n")]`.

## Expected Results

- Step 1: `Pending` at 6, true at 7, then false at 7; the stamp-3 observation in `late` with arrival 6 and anchor 0, never read.
- Step 2: late and on-time.
- Step 3: empty `late` list and a different result identity; `MissingHistory` without any late classification.
