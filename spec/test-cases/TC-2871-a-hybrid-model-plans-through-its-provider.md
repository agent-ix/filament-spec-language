---
id: TC-2871
title: "A hybrid model plans through its negotiated provider"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1021
    type: verifies
---
# TC-2871: A hybrid model plans through its negotiated provider

## Description

Verify hybrid plan synthesis outcomes. Scope: FR-1021-AC-1 and AC-2.

## Test Procedure

1. Through `run_with_providers` with the reference hybrid provider, plan the tank toward `Drain` and toward `h >= 12`; through `run`, which holds no hybrid provider, plan toward `Drain`; send a request without `dwell_grid`.
2. Send `dwell_grid` `1` and the Rational `0/1`, and a `synthesis` member carrying `engines`.

## Expected Results

- Step 1: `feasible`; `infeasible` certified; `unsupported` naming `hybrid-dynamics`, exit 21; `missing-member` at `/synthesis/dwell_grid`.
- Step 2: `wrong-value-kind` and `invalid-value` at `/synthesis/dwell_grid`; `unknown-member` at `/synthesis/engines`; every member present on the `unsupported` outcome, the no-plan members `null`.
