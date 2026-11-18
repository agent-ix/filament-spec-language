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

Verify that the export takes bounds only from `bound` declarations, read through `CheckedGraph::population_bound` whether or not they are walked, that it names the `[N]` region, and that it states `Init` from declared initial constraints. Scope: FR-1035-AC-1 to AC-4.

## Test Procedure

1. Export the spine unit with `bound HistoryBound on Config::config_history [8]` and no reachability, then without the bound.
2. Export the spine unit's `Low` with no `initial`, then with `constraint Small` added, and `SlowEnough` over `Braking` with no `initial`.
3. Read `CheckedGraph::population_bound` for the `config_history` entry with and without the bound.
4. Read the `fsl:` comment preceding `TypeOK` in the bounded export.

## Expected Results

- Step 1: `TypeOK` holds a `Cardinality(DOMAIN v_…) <= 8` conjunct. Without the bound, there is no `Cardinality`.
- Step 2: `Init == TypeOK` with exit 0. With `Small` added, `Init` holds no `<= 3`. `Braking`'s `Init` names `Moving`.
- Step 3: `None` without the bound. With the bound, `maximum` 8, label `HistoryBound` and the region of `[8]`.
- Step 4: the comment's region is `<authority>/<identity>:<start>-<end>` over the bytes `[8]`.
