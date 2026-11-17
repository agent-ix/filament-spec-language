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

Verify hybrid plan synthesis outcomes. Scope: FR-1021-AC-1.

## Test Procedure

1. Plan the tank toward `Drain`, toward `h >= 12`, with no provider, and without `dwell_grid`.

## Expected Results

- Step 1: `feasible`; `infeasible` certified; `unsupported`; refusal at `/synthesis/dwell_grid`.
