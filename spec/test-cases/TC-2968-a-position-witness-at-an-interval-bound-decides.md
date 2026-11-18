---
id: TC-2968
title: "A position witness at an interval bound decides"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: verifies
---
# TC-2968: A position witness at an interval bound decides

## Description

Verify that a position-counted reading is exact, so the generated monitor
decides a witness at a distance equal to an interval bound as the batch
evaluator does. Scope: FR-251-AC-54.

## Test Procedure

1. Under infinite-trace, plan `once[3,*] holds(p)` with `p` true at position 0 and run the generated entries over positions 0 to 3.
2. Under infinite-trace, plan `eventually[2,*] holds(p)` with `p` true at positions 1 and 2 and run the entries from origin 0.
3. Under event-position, plan FR-251-AC-52's `eventually[0, 3]` clause with the activation at step 0 and `c.versionNumber = 3` first at step 3.

Test: `fsl-analyze/tests/it/monitor_plan.rs`.

## Expected Results

- Step 1: `indeterminate` at position 2, `true` at position 3.
- Step 2: not discharged at position 1; discharged `true` at position 2.
- Step 3: the step-0 obligation settles `true` at step 3 and is never `indeterminate`.
