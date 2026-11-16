---
id: TC-2385
title: "An event-position model-steps clause is planned with a position counter"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: verifies
---
# TC-2385: An event-position model-steps clause is planned with a position counter

## Description

An event-position model-steps clause is planned with a position counter.

Scope: FR-251-AC-52.

## Test Procedure

Produce the plan of the event-position `model-steps` deadline clause with a `step` target. Test: `fsl-analyze/tests/it/monitor_plan.rs` `a_non_timed_clause_is_planned_with_a_position_counter`.

Tag the tests `#[trace("TC-2385", "FR-251-AC-52")]`.

## Expected Results

- The plan is produced, its one interval is `[0, 3]` and its buffer capacity 5.
