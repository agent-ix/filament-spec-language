---
id: TC-2064
title: "Weighted steps under a workload with weight 0"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-187
    type: verifies
---
# TC-2064: Weighted steps under a workload with weight 0

## Description

Verify FR-187's step probabilities under a workload that gives some or every operation weight 0, and the admission check of rewards in a package read from the wire.

Scope: FR-187-AC-5.

## Test Procedure

1. Under `Service` with `Server::reset: 0`, enumerate every reachable state and call `weighted_steps`.
2. Repeat with weight 0 on every operation and compare with weight 1 on every operation.
3. Admit a hand-built checked package read from the wire whose reward is `n - 3` over `n: Int[0, 5]`.

Tag the tests `#[trace("TC-2064", "FR-187-AC-5")]`.

## Expected Results

- Step 1: no `reset` step wherever another operation is enabled; `reset` steps sum to 1 where it is alone; every non-terminal state sums to 1.
- Step 2: equal steps at every state.
- Step 3: admission refused `negative-reward` naming `n = 0`.
