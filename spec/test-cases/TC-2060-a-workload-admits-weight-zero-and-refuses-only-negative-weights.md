---
id: TC-2060
title: "A workload admits weight 0 and refuses only negative weights"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: verifies
---
# TC-2060: A workload admits weight 0 and refuses only negative weights

## Description

Verify that S3 admits a workload weight of 0, including a workload of all zero weights, and refuses negative weights and omitted operations.

Scope: FR-185-AC-3, FR-185-AC-10.

## Test Procedure

Fixtures: ADR-024 §7.2's `Service` and its workload `Steady`.

1. Check `Steady` with `Server::reset: 0` and weight 1 on the other operations; read back the workload.
2. Check `Steady` with weight 0 on every operation.
3. Check `Steady` with `Server::request: rational(-1, 1)`, and `Steady` without `Server::reset`.

Tag the tests `#[trace("TC-2060", "FR-185-AC-n")]`.

## Expected Results

- Step 1: checks with no diagnostic; weight 0 recorded for `Server::reset`.
- Step 2: checks.
- Step 3: `invalid-weight` naming `Steady` and `Server::request`; `incomplete-workload` naming `Steady` and `Server::reset`.
