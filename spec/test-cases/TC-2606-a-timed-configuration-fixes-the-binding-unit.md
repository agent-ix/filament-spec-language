---
id: TC-2606
title: "A timed configuration fixes the binding unit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-933
    type: verifies
---
# TC-2606: A timed configuration fixes the binding unit

## Description

A `timed` configuration's unit converts every bound, unit-free bounds being
read in it, and a unit that is not a declared time unit is refused. Scope:
FR-933-AC-2, FR-933-AC-3.

## Test Procedure

1. Compile `eventually[0, 200]` and `eventually[0 ms, 200 ms]` with `"unit": "ms"`; compile the second with `"unit": "us"`.
2. Compile with `"unit": "minute"` (undeclared), with `"unit": "m"` (a `Length` unit), and a clause with the bound `200 m`.

Test: `tests/it/timed_clock_unit.rs` `a_timed_configuration_fixes_the_binding_unit`.

## Expected Results

- Step 1: equal bounds and keys under `ms`; bounds `0`, `200000` under `us`, with a different key.
- Step 2: `invalid-request` at stage `emit`, exit 20, `cause` `unknown-clock-unit`, then `clock-unit-dimension`; `ill_typed`/`type-mismatch` at `200 m`.
