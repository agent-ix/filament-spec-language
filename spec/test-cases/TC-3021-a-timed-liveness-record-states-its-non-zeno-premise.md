---
id: TC-3021
title: "A timed liveness record states its non-Zeno premise"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-235
    type: verifies
---
# TC-3021: A timed liveness record states its non-Zeno premise

## Description

Verify `ZoneCheckRecord::non_zeno` and its FSpec FR-416 wire member on each settled liveness-product record and its absence on other records. Scope: FR-235-AC-7 and AC-8.

## Test Procedure

1. Settle FR-240-AC-1's response claim over `Rpc` with `T = 4 ms`, its `reply` self-loop variant and its `echo` variant through `settle_zone_check`.
2. Settle `always eventually holds(true)` over `Stall`, and `NoLateReply` over `Rpc`.
3. Run `fsl analyze` over FR-632-AC-2's `Within` and `Early` on the zone and digital-clock routes.

## Expected Results

- Step 1: `NotRequired`, `Assumed`, then `Checked` on the refutation.
- Step 2: `Inconclusive(NoAdmittedBehaviour)` with `Refuted`; `NoLateReply` with `None`.
- Step 3: `Within` carries the same `non_zeno` on both routes, `not-required` or `assumed`; `Early` carries `checked` on both.
