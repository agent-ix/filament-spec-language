---
id: TC-2025
title: "Liveness over an ra parallel is decided under visibility fairness"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-228
    type: verifies
---
# TC-2025: Liveness over an ra parallel is decided under visibility fairness

## Description

Verify that a liveness clause over a protocol subject with an `ra` `parallel` is decided, with visibility fairness in its fairness set, and never refused as unsupported. Scope: FR-228-AC-11.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | FR-212's `Chatter` with `Chatter::Both` resolved to `ra`, `eventually holds(k.v = 1)` | `Holds`, exploration complete; fairness set holds one strong `memory` visibility member and three `scheduler` constraints |
| 2 | FR-228-AC-2's protocol resolved to `ra`, `eventually holds(c.r = 1)` | `proved`; the stale-load lasso is rejected by visibility fairness |

## Expected Results

- No run returns an unsupported-construct refusal.
