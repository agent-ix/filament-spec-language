---
id: TC-1586
title: "Deadlock-freedom and time-lock items respect intended terminal states"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-626
    type: verifies
---
# TC-1586: Deadlock-freedom and time-lock items respect intended terminal states

## Description

Verify FR-626's derived items. Scope: FR-626-AC-1 to FR-626-AC-3.

## Test Procedure

1. Analyze `{"claim": "M", "item_kind": "deadlock-freedom"}` over the Counter fixture.
2. Repeat with `terminal Done using v on M when { self.value = 3 };`.
3. Analyze `time-lock-freedom` over the Counter subject and over a timed subject that blocks time.

## Expected Results

- Step 1: `refuted`, last state `value = 3`.
- Step 2: `proved`.
- Step 3: `invalid-request` `inapplicable-kind`; `refuted` with a concretised counterexample.
- Assertions compare the outcome document's typed members and the exit code, never message text.
