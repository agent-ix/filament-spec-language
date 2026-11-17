---
id: TC-2536
title: "Refinement liveness through hidden abstract fields is decided"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-899
    type: verifies
---
# TC-2536: Refinement liveness through hidden abstract fields is decided

## Description

Verify that refinement liveness through hidden abstract fields is decided. Scope: FR-899-AC-1, FR-899-AC-2.

Catches a check that requires every candidate run to be fair, or answers `unsupported`.

## Test Procedure

1. Check `RegisterHistory` without `prev` and with `ensure fair weak write`, its peek-loop variant, and the variant with one fair candidate run.

## Expected Results

- Step 1: The first holds; the peek loop is `Violated`, `HiddenUnfair{write}`; the one-fair-candidate variant holds.
