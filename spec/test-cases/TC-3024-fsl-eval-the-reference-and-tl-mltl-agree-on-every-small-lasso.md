---
id: TC-3024
title: "fsl-eval, the reference and tl-mltl agree on every small lasso"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-932
    type: verifies
---
# TC-3024: fsl-eval, the reference and tl-mltl agree on every small lasso

## Description

Verify the lasso half of the three-way differential: the past cases under
`filament.temporal.infinite-trace/v1`, over every small lasso, including the
pre-origin regression cases. Scope: FR-932-AC-4 and FR-932-AC-5.

## Test Procedure

1. For each past case and each lasso of a prefix of 0 to 2 samples and a loop of 1 to 3, at most 3 samples, over 0, 1 and 2, evaluate the clause `on each` in fsl-eval and in the reference that unrolls the lasso and reads positions before 0 as false.
2. Run each case through `tl_lowering` with `cross_evaluate.trace` set to that lasso.
3. Over the lasso with no prefix and loop `0`, evaluate `once[1,1] not p` and `historically[0,2] p` at position 0 in all three, and in a reference that clips the past window at the origin.

Tests: `fsl-eval/tests/it/mltl_differential.rs` `fsl_eval_agrees_with_the_reference_on_every_lasso`; `tests/it/mltl_differential.rs` `tl_mltl_agrees_on_every_lasso`.

## Expected Results

- Step 1: equal outcomes for every case and lasso.
- Step 2: `supported` under `mltl.infinite-trace/v1` with the `bounded-past` classification, `agree` true, `faults` `[]`.
- Step 3: `true` and `false` at 0 in all three; the clipping reference disagrees on both.
