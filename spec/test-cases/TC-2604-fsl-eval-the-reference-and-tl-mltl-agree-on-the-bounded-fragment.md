---
id: TC-2604
title: "fsl-eval, the reference and tl-mltl agree on the bounded fragment"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-932
    type: verifies
---
# TC-2604: fsl-eval, the reference and tl-mltl agree on the bounded fragment

## Description

Verify the three-way differential over the bounded past and future corpus,
including the pre-origin regression cases. Scope: FR-932-AC-1 to FR-932-AC-3.

## Test Procedure

1. For each corpus case and each trace of length 1 to 4 over 0, 1, 2, evaluate the clause `on each` in fsl-eval and in the reference.
2. Run each case through `tl_lowering` with `cross_evaluate.trace` set to that trace.
3. Over `0, 0, 0`, evaluate `once[1,1] not p` and `historically[0,2] p` in all three, and in a reference that clips the past window at the origin.

Test: `tests/it/mltl_differential.rs` `fsl_eval_the_reference_and_tl_mltl_agree`.

## Expected Results

- Step 1: equal outcomes for every case and trace.
- Step 2: past and future cases `supported`, `agree` true, `faults` `[]`; the mixed case `unsupported`, `dimensions` `["direction"]`.
- Step 3: `true` at 0 and `false` at 0 in all three; the clipping reference disagrees on both.
