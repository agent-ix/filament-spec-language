---
id: TC-1931
title: "Evaluation memory per work unit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-130
    type: verifies
---
# TC-1931: Evaluation memory per work unit

## Description

Verify NFR-130-AC-2: evaluation memory stays within 1 KiB per charged work unit.

## Test Procedure

Call `d` of NFR-130-AC-1 with `n = 1000000` on an unlimited meter, measuring the growth of the peak resident set across the call, and divide by the consumed `work_units` and by the largest live-occurrence count reached (both limits unlimited). Test: `fsl-eval/tests/it/evaluation_rulings.rs`, run alone on Linux, measuring the growth of the process peak resident set (`VmHWM`).

## Expected Results

Both ratios are at most 1 KiB.
