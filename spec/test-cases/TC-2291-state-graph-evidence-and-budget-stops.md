---
id: TC-2291
title: "State-graph evidence uses typed keys and loci, and budget stops settle by form"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: verifies
---
# TC-2291: State-graph evidence uses typed keys and loci, and budget stops settle by form

## Description

Verify the state-graph evidence encoding and the budget rule of a state-graph
item: the instance key is the typed canonical universe key, the `undefined`
arm carries its locus, a stopped `possible` item with witnesses is
`inconclusive` `well-definedness-unchecked`, and a stopped `always possible`
item is `incomplete`. Scope: FR-628-AC-4, FR-628-AC-5.

## Test Procedure

1. Analyze `possible (c.value = 3)` over the Counter fixture and read the evidence `instance`.
2. Analyze a `possible` claim whose target divides by `3 - c.value` and read the `undefined` arm.
3. Analyze `possible (c.value = 3)` and `always possible (c.value = 0)` with `temporal.limits.max_states` 2.

## Expected Results

- Step 1: `instance` is `{"type": "instance", "parameter": "c", "key": K}`, `K` counter `a`'s FSpec FR-181 typed canonical universe key, not an object reference.
- Step 2: `refuted`; the `undefined` arm's `where` carries `position`, `state_key`, `predicate` `"P"` and the division's `locus`.
- Step 3: `inconclusive` `well-definedness-unchecked` for the `possible` item, never `proved` or `incomplete`; `incomplete` `LimitReached{max_states}` for the `always possible` item.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`state_graph_evidence_keys_loci_and_budget_stops`.
