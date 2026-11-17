---
id: TC-2392
title: "A measured recursive clock reading unfolds at a constant call"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-2392: A measured recursive clock reading unfolds at a constant call

## Description

A measured recursive clock reading unfolds at a constant call.

Scope: FR-230-AC-68.

## Test Procedure

Check `loop(self.x, 3)` and `loop(self.x, self.n)` in a precondition, then check `loop(self.x, 3)` again with the work budget one unit below the work the admitted check used. Test: `fsl-semantics/tests/it/timed_model.rs` `a_measured_recursive_clock_reading_unfolds`.

Tag the tests `#[trace("TC-2392", "FR-230-AC-68")]`.

## Expected Results

- The constant call checks with no `TimedNotChecked`; the data call refuses `clock-read-outside-constraint`.
- The reduced budget refuses `stage_limit_exceeded`/`work-budget-exceeded` at the call.
