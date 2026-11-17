---
id: TC-2802
title: "In-process replay of a protocol conformance counterexample"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-991
    type: verifies
---
# TC-2802: In-process replay of a protocol conformance counterexample

## Description

Verify that `fsl_analyze::protocol_analysis::conformance::replay_counterexample` is published and replays an every-behaviour counterexample from the protocol subject alone, with coded refusals. Scope: FR-991-AC-1, FR-991-AC-2, FR-991-AC-3.

Catches a replay that needs source text, a crate-private refusal map, and an item `replay` member that disagrees with the published function.

## Test Procedure

1. Build the protocol subject of the order fixture without its `receive` and replay the item's deadlock counterexample.
2. Replay it with the last step's transition identity changed, and with an extra enabled step appended.

## Expected Results

- Step 1: `Reproduced`, equal to the item's `replay` member.
- Step 2: refusal `invalid_runtime_input`/`invalid-value` at that step's index; `ReplayParity`.
