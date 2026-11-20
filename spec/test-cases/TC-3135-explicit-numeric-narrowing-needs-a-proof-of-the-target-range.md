---
id: TC-3135
title: "Explicit numeric narrowing needs a proof of the target range"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: verifies
---
# TC-3135: Explicit numeric narrowing needs a proof of the target range

## Description

Verify that a standalone `convert<T>(x)` that narrows an integer, rational or decimal source checks and lowers to a narrowing operation only under a proof that the value lies in `T`, and otherwise refuses with the target range.

Scope: FR-093-AC-67.

## Test Procedure

1. Check each narrowing of FR-093-AC-67 under its guard or literal and read the lowered operation.
2. Check the same narrowings without the proof and read the refusal.

Tag the tests `#[trace("FR-093-AC-67", "TC-3135")]`.

## Expected Results

- Step 1: each lowers to `filament.op.numeric.narrow` or `filament.op.rational.narrow` with its target member and no mode.
- Step 2: each refuses `undefined_expression`/`unproved-range`, and an unproved integer or decimal target carries the target's integer range as the required interval.
