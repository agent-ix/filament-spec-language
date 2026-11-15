---
id: TC-1588
title: "State-graph items report witnesses and counterexamples"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: verifies
---
# TC-1588: State-graph items report witnesses and counterexamples

## Description

Verify FR-628's request and result. Scope: FR-628-AC-1 to FR-628-AC-3.

## Test Procedure

1. Analyze `possible (value = 3)` and `possible (value = 4)` over the Counter fixture.
2. Analyze `always possible (value = 0)`.
3. Analyze a `possible` claim with `witness_samples` 3 and seed 7 twice.

## Expected Results

- Step 1: `proved` with a replayed three-step witness; `refuted`.
- Step 2: `refuted` with a path to `value = 1`.
- Step 3: three equal witnesses across the runs.
- Assertions compare the outcome document's typed members and the exit code, never message text.
