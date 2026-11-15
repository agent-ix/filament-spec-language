---
id: TC-1908
title: "Operation contract and type invariants through run"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-702
    type: verifies
---
# TC-1908: Operation contract and type invariants through run

## Description

Verify FR-702-AC-1 to AC-3: the `frame` run evaluates the operation's own `requires` and `ensures`, and FR-652's `value_invariants` items selection evaluates the model's type invariants.

## Test Procedure

Run the ConfigVersion `frame` selection over the invocation of FR-702-AC-1 with each domain contract variant; run the `items` request with FR-652's `value_invariants` selection of AC-3. Tests: `fsl-replay/src/spine/clause/tests/operation_contract_run.rs` (AC-1, AC-2) and `tests/it/value_invariants_run.rs` (AC-3).

## Expected Results

Each false clause gives `violation`, `truth: false`, exit 10, naming the clause; each holding case gives success with `truth: true`.
