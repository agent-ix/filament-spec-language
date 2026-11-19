---
id: TC-2292
title: "Several instances report the first refuted one, and a state-graph refutation names its claim"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-2292: Several instances report the first refuted one, and a state-graph refutation names its claim

## Description

Verify how an item over several claim instances reports one record (the
first refuted instance in universe-key order), and that a refuted
state-graph item names its claim as the replay target. Scope: FR-621-AC-9.

## Test Procedure

1. Analyze `always holds(c.value >= 1)` over the Counter subject with counters `a` at 1, `b` at 0 and `d` at 0, twice.
2. Analyze `always possible (c.value = 0)` over the one-counter fixture.

## Expected Results

- Step 1: `refuted`; `counterexample.over_binding` names `b`; the two runs give equal items.
- Step 2: `refuted`; `replay.target` is `{"state_clause": Q}`, `Q` ending in the claim's declared name; `replay.reproduced` `true`.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`several_instances_report_the_first_refuted_and_state_graph_replay_names_the_claim`.
