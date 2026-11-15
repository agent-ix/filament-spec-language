---
id: TC-1907
title: "Set and bag fold step catalog"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-701
    type: verifies
---
# TC-1907: Set and bag fold step catalog

## Description

Verify FR-701-AC-1 and AC-2: a set or bag fold admits exactly the FSpec FR-145 step catalog, over any element type.

## Test Procedure

Check each fold of FR-701-AC-1 and AC-2 as a function body. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

The catalog steps check, the other set steps refuse `ill_typed`/`operator-ineligible`, and the subtraction over a sequence checks.
