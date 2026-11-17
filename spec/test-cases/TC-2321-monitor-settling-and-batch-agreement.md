---
id: TC-2321
title: "A monitored clause settles when its evaluator decides it and keeps its deciding report"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: verifies
---
# TC-2321: A monitored clause settles when its evaluator decides it and keeps its deciding report

## Description

Verify FR-643's batch agreement for settled clauses and its one settling rule
across profiles. Scope: FR-643-AC-6, FR-643-AC-7.

## Test Procedure

1. Step a `Finite` session over samples 0, 1, 2, 1 for `Never` and `Reaches`; compute `B(3)` and `B(4)`.
2. Step `E` (`eventually[0,2] holds(r.level = 2)`, event-position) over 0, 1, 2 and over 0, 1, 1.

## Expected Results

- Step 1: `Never` settles `violation` at step 3 and reports `B(3)` (three-sample counterexample) at step 4; `B(4)` has category `violation`, truth `false` and four samples; `Reaches` stays `Pending` at every step, reporting `B(k)`, and `finish` returns `B(4)` for it.
- Step 2: `E` is `Pending` at steps 1 and 2 although `B(1)` and `B(2)` are `violation`. Over 0, 1, 2 it settles at step 3 with report exactly `B(3)` (`success`, basis `closed-scope`, no `finite_window`, no ledger) and `Settlement` `{step: "3", trigger: "decided", basis: "decisive-witness", horizon: "2", position: "2"}`; over 0, 1, 1 with report `B(3)` (`violation`, `closed-scope`) and `Settlement` basis `decisive-counterexample`, horizon "2", position "0". `finish` over 0, 1 gives `Settlement` `{step: "2", trigger: "finish"}` and report `B(2)`.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`.
