---
id: TC-2868
title: "Exports match golden files and optional oracles"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1018
    type: verifies
---
# TC-2868: Exports match golden files and optional oracles

## Description

Verify the golden comparison and the optional TLC oracle. Scope: FR-1018-AC-1 and AC-2.

## Test Procedure

1. Export FR-756-AC-1's counter (`inc`, `value <= 5`) under both profiles and compare with its golden files.
2. Run TLC on the TLA+ export when present, and `analyze` the same model.

## Expected Results

- Step 1: byte-equal.
- Step 2: TLC's state count and verdict equal the analyze run's `exploration.states` (six) and verdict, or `tlc not found` with step 1 still run.
