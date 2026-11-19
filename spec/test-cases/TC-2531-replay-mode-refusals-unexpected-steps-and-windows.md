---
id: TC-2531
title: "Replay mode refuses unreadable recordings and reports unexpected steps and windows"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: verifies
---
# TC-2531: Replay mode refuses unreadable recordings and reports unexpected steps and windows

## Description

Verify that replay mode refuses unreadable recordings and reports unexpected steps and windows. Scope: FR-895-AC-4, FR-895-AC-5, FR-895-AC-6.

Catches a recording defect reported as behaviour, or behaviour refused as a defect.

## Test Procedure

1. Assess a recording naming `Courier`; one with a send on `Tell` where none is enabled; one whose attempt reads 130 on clock `c` under `within [0, 120]`, then with the reading absent; assess each in batch and incrementally.

## Expected Results

- Step 1: `Courier` is refused `missing-name` with its index; the send is `unexpected` and `refuted`; the window is `violated`, then `pending`; batch and incremental verdicts are equal.
