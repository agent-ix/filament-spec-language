---
id: TC-2870
title: "Previous and interval clauses transfer only through step-for-step refinements"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1020
    type: verifies
---
# TC-2870: Previous and interval clauses transfer only through step-for-step refinements

## Description

Verify the step-for-step transfer rule. Scope: FR-1020-AC-1 and AC-2.

## Test Procedure

1. Transfer the two clauses through a step-for-step refinement.
2. Transfer them through a refinement with a `stutter` arm, an `any` arm and a dense-time concrete subject.

## Expected Results

- Step 1: both `proved`, `decisive-witness`, no exploration.
- Step 2: `inconclusive`, `transfer-not-preserving` naming the operators.
