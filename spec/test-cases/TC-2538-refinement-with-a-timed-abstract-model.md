---
id: TC-2538
title: "A refinement whose abstract model is timed is decided and certified"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-900
    type: verifies
---
# TC-2538: A refinement whose abstract model is timed is decided and certified

## Description

Verify that a refinement whose abstract model is timed is decided and certified. Scope: FR-900-AC-1, FR-900-AC-2, FR-900-AC-3.

Catches abstract clocks guessed instead of derived from the step map.

## Test Procedure

1. Check `TReq2` against timed `TReq`; then the `x >= 0` variant; then an abstract invariant `y <= 3` against a concrete model that waits 4.

## Expected Results

- Step 1: The safety half is `proved` with a certificate over auxiliary clock `y` that FR-245 accepts; `AbstractClockGuard{send}` with a reproducing prefix that fails when its delay is set to 1; `AbstractTimeInvariant` at the delay crossing 3.
