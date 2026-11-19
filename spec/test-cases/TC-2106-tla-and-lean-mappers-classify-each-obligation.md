---
id: TC-2106
title: "The TLA+ and Lean mappers classify each obligation"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-756
    type: verifies
---
# TC-2106: The TLA+ and Lean mappers classify each obligation

## Description

Verify the two output mappers and their fidelity dispositions. Scope: FR-756-AC-1 and FR-756-AC-2.

## Test Procedure

1. Export the counter model with `inc` and `value <= 5` under each profile, twice.
2. Export a dense-time clause and `Braking`'s flows.

## Expected Results

- Step 1: every obligation `preserved`, every region commented with its identity and span; byte-equal exports.
- Step 2: `conditional` (non-Zeno) and `unrepresented`; neither `preserved`.
