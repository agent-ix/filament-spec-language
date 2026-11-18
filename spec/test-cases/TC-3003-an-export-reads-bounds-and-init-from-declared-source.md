---
id: TC-3003
title: "An export reads population bounds and Init from declared source"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1035
    type: verifies
---
# TC-3003: An export reads population bounds and Init from declared source

## Description

Verify that the export takes bounds only from `bound` declarations, whether or not they are walked, and states `Init` from declared initial constraints. Scope: FR-1035-AC-1 and AC-2.

## Test Procedure

1. Export the spine unit with `bound HistoryBound on Config::config_history [8]` and no reachability, then without the bound.
2. Export the spine unit's `Low` with no `initial`, and `SlowEnough` over `Braking` with no `initial`.

## Expected Results

- Step 1: `TypeOK` holds a `Cardinality(DOMAIN v_…) <= 8` conjunct. Without the bound, there is no `Cardinality`.
- Step 2: `Init == TypeOK` with exit 0. `Braking`'s `Init` names `Moving`.
