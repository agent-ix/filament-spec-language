---
id: TC-2107
title: "A supported temporal clause lowers to a TL formula that agrees with native evaluation"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-757
    type: verifies
---
# TC-2107: A supported temporal clause lowers to a TL formula that agrees with native evaluation

## Description

Verify the lowering to `tl-syntax` and the `tl-mltl` cross-evaluation. Scope: FR-757-AC-1 to FR-757-AC-3.

## Test Procedure

1. Lower `always [0, 3] holds(c.value <= 2)` under the event-position false-extension profile.
2. Cross-evaluate over the fixed trace corpus, then with one atom mis-mapped.
3. Lower a clause under a timed profile.

## Expected Results

- Step 1: `G[0,3] p0` under `mltl.closed-trace/v1`, `p0` naming the `holds` leaf.
- Step 2: agreement at every activation position; a `correspondence-loss` fault naming clause and position.
- Step 3: the unsupported disposition and no formula.
