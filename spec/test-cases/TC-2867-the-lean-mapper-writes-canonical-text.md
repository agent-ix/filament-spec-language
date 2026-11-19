---
id: TC-2867
title: "The Lean mapper writes canonical, proof-free text"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1017
    type: verifies
---
# TC-2867: The Lean mapper writes canonical, proof-free text

## Description

Verify the Lean file layout. Scope: FR-1017-AC-1.

## Test Procedure

1. Export `Low` under `filament.output.lean4/v1`.

## Expected Results

- Step 1: the stated definitions, no `theorem` or `sorry`.
