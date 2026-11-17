---
id: TC-2803
title: "In-process replay of a recording counterexample"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-991
    type: verifies
---
# TC-2803: In-process replay of a recording counterexample

## Description

Verify that a recording-mode counterexample replays by re-assessing its prefix. Scope: FR-991-AC-4.

Catches a replay that accepts a prefix not ending at the first violation, and an admission defect reported without its code.

## Test Procedure

1. Replay FR-895-AC-5's recording with `trace_position` at the unexpected `Tell` send, then one earlier.
2. Replay a recording naming the undeclared role `Courier`.

## Expected Results

- Step 1: `Reproduced`; `ReplayParity`.
- Step 2: refusal `invalid_runtime_input`/`missing-name`.
