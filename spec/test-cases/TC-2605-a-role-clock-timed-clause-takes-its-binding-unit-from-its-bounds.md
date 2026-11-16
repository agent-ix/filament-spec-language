---
id: TC-2605
title: "A role-clock timed clause takes its binding unit from its bounds"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-933
    type: verifies
---
# TC-2605: A role-clock timed clause takes its binding unit from its bounds

## Description

With no `timed` configuration, a role-clock timed clause reads its bounds in
the coarsest unit they name, or as plain numbers when none names a unit.
Scope: FR-933-AC-1.

## Test Procedure

1. Check in process `Ack` with `eventually[0 ms, 200 ms]`.
2. Check `eventually[0 ms, 500 us] holds(r.ack)`; evaluate over stamps `0`, `1/3` and `0`, `2/3` with `ack` at the second.
3. Check `eventually[0, 200] holds(r.ack)`; evaluate over stamps `0`, `150` with `ack` at the second.

Test: `fsl-eval/tests/it/timed_recorded_trace.rs` `a_role_clock_clause_reads_its_bounds_in_its_binding_unit`.

## Expected Results

- Step 1: unit `ms`, bounds `0`, `200`.
- Step 2: unit `ms`, bounds `0`, `1/2`; holds; violated at position 0.
- Step 3: no named unit, bounds `0`, `200`; holds.
