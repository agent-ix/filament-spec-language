---
id: TC-2540
title: "History, whole-population and seq rows are written and read back"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-901
    type: verifies
---
# TC-2540: History, whole-population and seq rows are written and read back

## Description

Verify that history, whole-population and seq rows are written and read back. Scope: FR-901-AC-1, FR-901-AC-2.

Catches the `RefinementNotEmitted` refusal and a lossy round trip.

## Test Procedure

1. Compile each adverse-corpus refinement case with a history, population or `seq` row; read each back and re-emit it.

## Expected Results

- Step 1: Each compiles, is admitted and holds the row as FR-901's table states; read-back equals the compiled refinement, and re-emission gives the same bytes.
