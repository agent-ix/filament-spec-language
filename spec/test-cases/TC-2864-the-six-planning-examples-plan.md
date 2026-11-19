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

Verify the six published planning examples. Scope: FR-1014-AC-1 and AC-2.

## Test Procedure

1. Materialize each case's published fixture files from `filament_specification::published_files("proposals/planning-v1/fixtures/<case>/")` beside its `examples/planning/<case>/request.json` and run it.
2. Evaluate every returned state against its rules.
3. List each example directory and compare its `synthesis` member with the published `plan.json`.

## Expected Results

- Step 1: each FSpec FR-810 result and exact objective.
- Step 2: every rule holds.
- Step 3: only `request.json` present; the members equal.
