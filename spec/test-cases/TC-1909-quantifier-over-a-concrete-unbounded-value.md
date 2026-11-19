---
id: TC-1909
title: "Quantifier over a concrete unbounded value"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-335
    type: verifies
---
# TC-1909: Quantifier over a concrete unbounded value

## Description

Verify FR-335-AC-6: a quantifier over a concrete value of an unbounded collection type is evaluated by visiting that value, and the result is never a claim settlement.

## Test Procedure

Call `p` of FR-335-AC-6 on each concrete set and on a short meter. Test: `fsl-eval/tests/it/evaluation_rulings.rs`.

## Expected Results

The results, visit charges and `Incomplete` outcome match FR-335-AC-6.
