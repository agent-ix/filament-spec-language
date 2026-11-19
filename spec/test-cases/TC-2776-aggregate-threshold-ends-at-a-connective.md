---
id: TC-2776
title: "A window-aggregate threshold ends at and, or and implies"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-982
    type: verifies
---
# TC-2776: A window-aggregate threshold ends at and, or and implies

## Description

A window-aggregate threshold ends at and, or and implies.

Scope: FR-982-AC-1, FR-982-AC-2.

## Test Procedure

Parse the formulas of FR-982-AC-1 and FR-982-AC-2. Test: `fsl-cst/tests/it/aggregate_connectives.rs`.

## Expected Results

- One atom ending at `1` (or at `s.k + 1`) under a conjunction, disjunction and implication node.
- `invalid_syntax`/`unexpected-token` at the `s` of `s.ok` for each connective.
