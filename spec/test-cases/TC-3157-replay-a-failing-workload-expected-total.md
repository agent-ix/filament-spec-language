---
id: TC-3157
title: "Replay a failing workload expected total"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3157: Replay a failing workload expected total

## Description

Verify FR-202-AC-32: a false expected-reward claim under a resolved workload
is refuted with a lower certificate checked in the counterexample role.

## Test Procedure

Run the expected-total case with workload transition weights 99:1 and claim
`expected total <= 1`. Inspect the emitted subsystem certificate and replay it
against the same checked package and workload.

## Expected Results

No scheduler is emitted. The certificate is `lower`, its independently
checked initial bound is strictly greater than 1, and replay settles the item
`refuted`. A lower bound equal to 1, an unchecked certificate, or a bare
engine value produces no token.
