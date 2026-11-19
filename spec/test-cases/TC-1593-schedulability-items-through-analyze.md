---
id: TC-1593
title: "Schedulability items settle on EN-7 in closed form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-633
    type: verifies
---
# TC-1593: Schedulability items settle on EN-7 in closed form

## Description

Verify FR-633's request item and result. Scope: FR-633-AC-1 to FR-633-AC-3.

## Test Procedure

1. Analyze `schedulable CtlFp on Ctl under fixed_priority;`.
2. Analyze a response claim at 12 ms and at 11 ms.
3. Analyze `schedulable CtlEdf on Ctl under edf;`.

## Expected Results

- Step 1: `proved`, `closed-form` `fixed-priority`, `certified`, backend `en-7`.
- Step 2: `proved`; `refuted` naming the task, response 12 and deadline 11.
- Step 3: `proved`, `closed-form` `edf`.
- Assertions compare the outcome document's typed members and the exit code, never message text.
