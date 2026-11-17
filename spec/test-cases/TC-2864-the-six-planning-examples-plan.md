---
id: TC-2864
title: "The six reference planning examples plan as specified"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1014
    type: verifies
---
# TC-2864: The six reference planning examples plan as specified

## Description

Verify the six published planning examples. Scope: FR-1014-AC-1.

## Test Procedure

1. Run each example under `examples/planning/`.
2. Evaluate every returned state against its rules.

## Expected Results

- Step 1: each FSpec FR-810 result and exact objective.
- Step 2: every rule holds.
