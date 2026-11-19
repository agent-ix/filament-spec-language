---
id: TC-2990
title: "Observe-only steps keep the model state"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: verifies
---
# TC-2990: Observe-only steps keep the model state

## Description

Verify the `recommend` step, its recommendation record and that no engine advances shared state through an observe-only step. Scope: FR-206-AC-13.

## Test Procedure

1. Compute the reachable graph of the FR-218-AC-45 role clause, with an `attempt` by another role over an operation writing `ready`, from an `M::Actor` with `ready = false`.
2. Run a clause holding an `attempt` with `boundary` `observe-only`, then a clause of `receive`, `check` and `recommend` only.

## Expected Results

- Step 1: a `recommend` step at `Advise` carries a `recommendation` naming `Advise`, `Auditor` and `M::Actor::op` with its receiver and empty arguments; every successor of a step at `Advise` or at the observe-only `receive` has its predecessor's `semantic` key member and compensation registrations; the other role's `attempt` reaches `ready = true`.
- Step 2: the first run refuses before any step with the FR-218-AC-46 refusal at the attempt; the second runs with every step keeping the model state.
