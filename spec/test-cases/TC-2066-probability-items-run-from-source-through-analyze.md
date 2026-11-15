---
id: TC-2066
title: "Probability items run from source through analyze"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: verifies
---
# TC-2066: Probability items run from source through analyze

## Description

Verify every probability item class through a `native-analyze/1` request compiled from source.

Scope: FR-741-AC-1 to FR-741-AC-4.

## Test Procedure

1. Run the statistical items of FR-741-AC-1 with `workers` 1 and 4; replay each reported witness.
2. Run the exact items of FR-741-AC-2 and check each certificate with `fsl-probability-check` in a separate process.
3. Run the refuting exact item and the timed exact item of FR-741-AC-3.
4. Run the refused items of FR-741-AC-4.

Tag the tests `#[trace("TC-2066", "FR-741-AC-n")]`.

## Expected Results

Each item settles as its AC states, with the result members FR-741 lists.
