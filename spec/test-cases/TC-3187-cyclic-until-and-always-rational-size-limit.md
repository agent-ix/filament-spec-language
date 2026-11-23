---
id: TC-3187
title: "Cyclic until and always rational size limit"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3187: Cyclic until and always rational size limit

## Description

Verify FR-201-AC-65: an exact violated quantity that exceeds
`max_rational_bits` stops incomplete instead of being proved, refused or
certified by value bounds.

## Test Procedure

Analyze FR-201-AC-59's `probability >= 1/4 [holds(A) until holds(B)]` claim
with `max_rational_bits` 1, then with the default setting.

## Expected Results

With `max_rational_bits` 1 the item settles
`Incomplete(LimitReached{MaxRationalBits, 1, exact_prob.max_rational_bits})`
with no certificate, no truth value and no FR-1042 document; it does not
settle `proved`, `unsupported` or `failed`. With the default setting it
settles `proved`.
