---
id: TC-3204
title: "Given delays race under unequal weights"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-3204: Given delays race under unequal weights

## Description

Three identities with `discrete` delays race under a workload with unequal weights; a tie is ordered by the weights, not evenly and not by canonical order.

Scope: FR-204-AC-18.

## Test Procedure

Build `Trio` (FR-204-AC-18) under the workload `Skew` weighting `A` 1, `B` 2 and `C` 3, and decide the four exact claims; rebuild the product through the checker and compare its edges and values; submit the `t.b` `Exact` certificates computed with even ties (`11/96`) and with ties in canonical order (`0`). Test: `fsl-analyze/tests/it/digital_clocks.rs` `given_delays_race_under_unequal_weights`.

Tag the tests `#[trace("TC-3204", "FR-204-AC-18")]`.

## Expected Results

- Exactly four race edges and no other action (no `UnitDelay`, `FreeDelays` or `WorkloadResolve`) at the initial state, each `Identity(A)`: `A` at 1 ms `1/2`, `A` at 2 ms `53/192`, `B` at 2 ms `7/48`, `C` at 2 ms `5/64`.
- `t.b` `ExactValue{7/48}`, `t.c` `ExactValue{5/64}`, `t.a` within 3 ms `ExactValue{149/192}`, `t.a` within 1 ms `ExactValue{1/2}`, all `proved` at the stated thresholds.
- Both certificates (`11/96` and `0`) are rejected `InequalityFails` at the initial state.
