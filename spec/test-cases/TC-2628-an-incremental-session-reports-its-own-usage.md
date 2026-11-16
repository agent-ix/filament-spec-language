---
id: TC-2628
title: "An incremental monitor session reports its own per-step and cumulative usage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-2628: An incremental monitor session reports its own per-step and cumulative usage

## Description

Verify that a monitor session's clause report carries `usage` as a closed `{cumulative, step}` object over FR-283's usage kinds, that `usage` is excluded from batch equality while every verdict member is compared, and that meter limits are charged per advance.

Scope: FR-643-AC-12.

## Test Procedure

1. Open a `Finite` session over `always holds(c.value <= 1)`; `step` observations with `c.value` 0, 1, 1; read each report's `usage` and compare every other member with `B(k)` from FR-283's `monitor` over the same prefix.
2. Reopen the session with a per-step work limit one unit below the second call's `step` from Step 1; `step` the same three observations.

Tag the tests `#[trace("TC-2628", "FR-643-AC-12")]`.

## Expected Results

- Step 1: each `usage` has exactly `cumulative` and `step`, each with exactly FR-283's usage kinds; `step` is that call's charge and `cumulative` the running sum of `step`; category, truth, disposition, counterexample or witness, settlement and positions equal `B(k)`.
- Step 2: the first call reports as in Step 1; the clause settles `incomplete` with `trigger` `limit` at the second call.
