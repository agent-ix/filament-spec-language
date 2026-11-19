---
id: TC-2530
title: "Replay mode gives one verdict per obligation over a recording"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: verifies
---
# TC-2530: Replay mode gives one verdict per obligation over a recording

## Description

Verify that replay mode gives one verdict per obligation over a recording. Scope: FR-895-AC-1, FR-895-AC-2, FR-895-AC-3.

Catches an assessor that reads a missing observation as satisfied or declines a misordered recording.

## Test Procedure

1. Assess a closed recording of the order fixture; one whose `Placed` and `Got` lie on a causal cycle through the `Tell` correlation (FR-895-AC-2); the recording cut after `Got`, open and then closed.

## Expected Results

- Step 1: All obligations `satisfied` and `proved`; the causal-order obligation `violated`, `misordered`, naming both, and `refuted` with `trace_position` at `Got`'s index; the open cut leaves obligations `pending` and `inconclusive`, and the closed cut makes them `missing`.
