---
id: TC-2860
title: "A plan request writes the plan outcome document"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1010
    type: verifies
---
# TC-2860: A plan request writes the plan outcome document

## Description

Verify the plan outcome document, its exits and the strict request reader. Scope: FR-1010-AC-1 and AC-2.

## Test Procedure

1. Run an optimal plan request over the spine model.
2. Run an infeasible one and one with an unknown member.

## Expected Results

- Step 1: `filament.plan-outcome/v1` with its members, exit 0.
- Step 2: `plan` null, exit 10; `unknown-member`, exit 20.
