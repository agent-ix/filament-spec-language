---
id: TC-2980
title: "An until with no upper bound is fulfilled where it is kept pending with its witness"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1034
    type: verifies
---
# TC-2980: An until with no upper bound is fulfilled where it is kept pending with its witness

## Description

Verify the acceptance set of a lower-bound-only `until[q, *]` component: a
run that keeps the operator pending at positions holding both `ψ` and `χ`
satisfies the claim, and a run that never discharges it does not. Scope:
FR-1034-AC-4.

## Test Procedure

1. Translate `always (holds(c.level = 1) implies eventually[1 ms, *] holds(c.level = 1))` and run its automaton and the timed evaluator on the lasso stamped 0 ms, 1 ms at levels 1, 1, re-entering position 0 2 ms after it.
2. Translate `always (holds(c.level = 1) implies eventually[1 ms, *] holds(c.level = 2))` and run its automaton and the evaluator on the same lasso.

Test: `fsl-eval/tests/it/timed_trace.rs`.

## Expected Results

- Step 1: the automaton does not accept the lasso; the evaluator returns `true`.
- Step 2: the automaton accepts the lasso; the evaluator returns `false`.
