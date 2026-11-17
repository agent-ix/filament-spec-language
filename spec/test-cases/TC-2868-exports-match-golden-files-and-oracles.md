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

1. Export the counter and compare with its golden files.
2. Run TLC on it when present.

## Expected Results

- Step 1: byte-equal.
- Step 2: equal state count and verdict, or `tlc not found`.
