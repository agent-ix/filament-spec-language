---
id: TC-2605
title: "A role-clock timed clause takes its binding unit from its configuration"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-933
    type: verifies
---
# TC-2605: A role-clock timed clause takes its binding unit from its configuration

## Description

A role-clock timed clause checked in process with a `timed` configuration
reads every bound in its unit, and with none refuses `missing-clock-selection`.
Scope: FR-933-AC-1.

## Test Procedure

1. With `{"kind": "timed", "unit": "ms"}` for `plant`, check in process `Ack` with `eventually[0 ms, 200 ms]`.
2. Check `eventually[0 ms, 500 us] holds(r.ack)`; evaluate over stamps `0`, `1/3` and `0`, `2/3` with `ack` at the second.
3. Check `eventually[0, 200] holds(r.ack)`; evaluate over stamps `0`, `150` with `ack` at the second.
4. Check each clause with no configuration for `plant`.

Test: `fsl-eval/tests/it/timed_recorded_trace.rs` `a_role_clock_clause_reads_its_bounds_in_its_binding_unit`.

## Expected Results

- Step 1: unit `ms`, bounds `0`, `200`.
- Step 2: unit `ms`, bounds `0`, `1/2`; holds; violated at position 0.
- Step 3: bounds `0`, `200` in `ms`; holds over `0`, `150`, violated over `0`, `250`.
- Step 4: each refuses `missing-clock-selection` naming `plant`, `Ack` and the timed profile.
