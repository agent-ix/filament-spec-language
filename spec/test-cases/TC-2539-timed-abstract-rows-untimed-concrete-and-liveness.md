---
id: TC-2539
title: "Clock rows refuse, an untimed concrete model is read with free delays, and timed liveness refutes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-900
    type: verifies
---
# TC-2539: Clock rows refuse, an untimed concrete model is read with free delays, and timed liveness refutes

## Description

Verify that clock rows refuse, an untimed concrete model is read with free delays, and timed liveness refutes. Scope: FR-900-AC-4, FR-900-AC-5, FR-900-AC-6.

Catches a clock mapping row, or an untimed concrete model read as delaying 0.

## Test Procedure

1. Compile a row `y = self.x`; check untimed `Req` against timed `TReq`, then against `y <= 0` with `y >= 1`; check the timed fairness constraint over a delaying loop.

## Expected Results

- Step 1: The row refuses at the row naming `y`; `Req` holds, then the empty zone refutes; the timed lasso refutes with positive total delay.
