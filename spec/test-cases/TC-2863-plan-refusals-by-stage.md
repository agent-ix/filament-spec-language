---
id: TC-2863
title: "Plan refusals by stage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1013
    type: verifies
---
# TC-2863: Plan refusals by stage

## Description

Verify S3 plan refusals and request admission refusals. Scope: FR-1013-AC-1 and AC-2.

## Test Procedure

1. Check a plan with `domain` `Int[-1, 3]`.
2. Check a goal reading `Reach.elapsed` over the spine model.
3. Send `synthesis` members naming a value-type `synthesis`, the policy `Nowhere`, a policy over `Braking`, and, under the population bound `bound HistoryBound on Config::config_history [1]`, the two-object healthy snapshot.

## Expected Results

- Step 1: `ill_typed`/`type-mismatch`, exit 20.
- Step 2: `missing_declaration`/`missing-name` at `elapsed`, exit 20.
- Step 3: `missing-name` at `/synthesis/declaration` and `/synthesis/policy/0`, `type-mismatch` at `/synthesis/policy/0`, the snapshot's admission refusal at `/synthesis/initial`; each exit 20, no outcome.
