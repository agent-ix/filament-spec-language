---
id: TC-2537
title: "Refinement liveness under strong abstract fairness is decided and replays"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-899
    type: verifies
---
# TC-2537: Refinement liveness under strong abstract fairness is decided and replays

## Description

Verify that refinement liveness under strong abstract fairness is decided and replays. Scope: FR-899-AC-3, FR-899-AC-4, FR-899-AC-5.

Catches a weak reading applied to a strong constraint.

## Test Procedure

1. Check `Pair` with strong and with weak `flipB`, then with concrete strong fairness on `flipB`; replay the refutations and edited copies.

## Expected Results

- Step 1: Strong is `Violated`, `AbstractUnfair{flipB}`; weak holds; concrete strong fairness removes the loop and the refinement holds; the refutations reproduce and the edits do not.
