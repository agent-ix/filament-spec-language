---
id: TC-2293
title: "Simulation reports discovery-tree traces and counts choice points"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-634
    type: verifies
---
# TC-2293: Simulation reports discovery-tree traces and counts choice points

## Description

Verify which traces a simulation item reports and what its coverage counts
mean, in both modes. Scope: FR-634-AC-4, FR-634-AC-5.

## Test Procedure

1. Enumerate the staying Counter (FR-627's `setTo` self-loop at `value = 0`).
2. Sample the staying Counter with seed 5 and `samples` 3.

## Expected Results

- Step 1: `success`; `coverage` states 4, transitions 4, choices 1; exactly one trace, `value` 0, 1, 2, 3.
- Step 2: three traces with trace indices 0, 1, 2 in order, each starting at `value = 0`; `seed` 5; `coverage.states` at most 4.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`simulation_reports_discovery_tree_traces_and_choice_points`.
