---
id: TC-2863
title: "Plan refusals by stage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1013
    type: verifies
---
# TC-2863: Plan refusals by stage

## Description

Verify S3 plan refusals. Scope: FR-1013-AC-1.

## Test Procedure

1. Check a plan with `domain` `Int[-1, 3]`.
2. Check a goal reading `Reach.elapsed` over the spine model.

## Expected Results

- Step 1: `ill_typed`/`type-mismatch`, exit 20.
- Step 2: `missing_declaration`/`missing-name` at `elapsed`, exit 20.
