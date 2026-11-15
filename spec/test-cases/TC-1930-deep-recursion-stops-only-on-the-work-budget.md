---
id: TC-1930
title: "Deep recursion stops only on the work budget"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-130
    type: verifies
---
# TC-1930: Deep recursion stops only on the work budget

## Description

Verify NFR-130-AC-1: a deep recursion completes or stops `incomplete` at the work budget, with no depth or memory refusal.

## Test Procedure

Call `d` of NFR-130-AC-1 with `n = 200000` on an unlimited meter and on a short one. Test: `fsl-eval/tests/it/evaluation_rulings.rs`.

## Expected Results

The unlimited call returns `0`; the short call returns `Incomplete` naming `work_units`; no outcome names depth, frames or memory.
