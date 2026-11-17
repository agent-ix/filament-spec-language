---
id: TC-2815
title: "Array order never orders a recorded protocol execution"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: verifies
---
# TC-2815: Array order never orders a recorded protocol execution

## Description

Verify that replay mode orders observations only by local order joined with
correlations, so a recording that lists a receive before its send in the
array, with no cycle, conforms. Scope: FR-895-AC-7.

Catches an assessor that reads the `observations` array position as causal
order and reports a conforming recording as misordered.

## Test Procedure

1. Assess the closed AC-1 recording with `Got` listed before `Placed` in the
   array and the local orders and correlations unchanged; assess the AC-1
   recording itself.

## Expected Results

- Step 1: every obligation is `satisfied` and the item settles `proved` for
  both recordings, with equal verdicts.
