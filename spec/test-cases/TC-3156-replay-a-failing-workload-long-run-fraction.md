---
id: TC-3156
title: "Replay a failing workload long-run fraction"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3156: Replay a failing workload long-run fraction

## Description

Verify FR-202-AC-31: a false long-run claim under a resolved workload is
refuted only by replaying its long-run subsystem certificate.

## Test Procedure

Run the workload long-run fraction case with 99:1 transition weights and claim
`long-run fraction >= 0.995`. Inspect the emitted counterexample and replay it
against the same checked package and claim.

## Expected Results

The counterexample has no scheduler and carries a `long-run` certificate.
Replay rebuilds the workload chain, checks its components and gain-bias
equations, and confirms the initial value is below `0.995`. The item settles
`refuted` with matching reproduced evidence. A proof-side acceptance, equality
at `0.995`, or a bare engine value produces no reproduced token.
