---
id: TC-1906
title: "Rational products and sums by the interval rule"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-700
    type: verifies
---
# TC-1906: Rational products and sums by the interval rule

## Description

Verify FR-700-AC-1 to AC-3: rational results are proved by FSpec FR-146's numerator and denominator interval rule.

## Test Procedure

Check each function of FR-700-AC-1 to AC-3. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

The admitting cases check; each narrower result type refuses `undefined_expression`/`unproved-range` at the expression.
