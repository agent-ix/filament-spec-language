---
id: TC-2861
title: "The plan search names its plan_search and accounting limits"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1011
    type: verifies
---
# TC-2861: The plan search names its plan_search and accounting limits

## Description

Verify the plan search limits and the evaluation meter. Scope: FR-1011-AC-1 and AC-2.

## Test Procedure

1. Run with `max_work` 1, then `max_states` 1.
2. Run with a low `accounting.work_units`.

## Expected Results

- Step 1: `incomplete` naming `plan_search.max_work`, then `plan_search.max_states`, exit 22.
- Step 2: `incomplete` naming `accounting.work_units`.
