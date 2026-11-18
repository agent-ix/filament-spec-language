---
id: TC-2974
title: "A good prefix of an infinite-trace clause holds"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-328
    type: verifies
---
# TC-2974: A good prefix of an infinite-trace clause holds

## Description

Verify that a finite prefix every extension of which satisfies an
infinite-trace clause settles holds, that a bad prefix outside the safety
fragment and a prefix that settles nothing stay pending, and that the tick
monitor agrees. Scope: FR-328-AC-7.

## Test Procedure

1. Over the `Counter` prefix `c.value` 0, 1, 2 under infinite-trace, evaluate each clause of FR-328-AC-7.
2. Run FR-251's tick monitor over the same three positions for `eventually holds(c.value = 1)` and `always holds(c.value <= 5)`.

Test: `fsl-eval/tests/it/temporal_trace.rs`.

## Expected Results

- Step 1: the three good prefixes are `Completed(true)` at position 0; the other three clauses are `Pending`.
- Step 2: `true` and `Pending`, equal to step 1.
