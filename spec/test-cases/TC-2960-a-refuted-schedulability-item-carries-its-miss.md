---
id: TC-2960
title: "A refuted schedulability item carries its miss as its counterexample"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-2960: A refuted schedulability item carries its miss as its counterexample

## Description

Verify that the analyze document writes a refuted EN-7 item's miss as the
FSpec FR-419 `schedulability-miss` counterexample, with no replay member.
Scope: FR-621-AC-13.

## Test Procedure

1. Analyze `Ctl` with the third WCET 6, items `CtlFp` and `CtlEdf`.
2. Analyze the unchanged `Ctl` with the same items.

Test: `fsl-analyze/tests/it/schedulability_routing.rs`.

## Expected Results

- Step 1: two `refuted` items with the `response` and `utilization` misses,
  no `replay`, `last_stage` `S7`, exit 10.
- Step 2: two `proved` items with no `counterexample`.
