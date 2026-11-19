---
id: TC-2386
title: "Fixed-sample and infinite-trace clauses are planned"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: verifies
---
# TC-2386: Fixed-sample and infinite-trace clauses are planned

## Description

Fixed-sample and infinite-trace clauses are planned.

Scope: FR-251-AC-53.

## Test Procedure

Produce the plans of the AC-52 `on each` clause under the fixed-sample profile, and under the infinite-trace profile of both the `on each` clause with an unbounded `eventually` and the `on origin` clause with an outer unbounded `always`. Test: `fsl-analyze/tests/it/monitor_plan.rs` `fixed_sample_and_infinite_trace_clauses_are_planned`.

Tag the tests `#[trace("TC-2386", "FR-251-AC-53")]`.

## Expected Results

- All three plans are produced; the fixed-sample one has interval `[0, 3]` and capacity 5.
- The infinite-trace `on origin` plan keeps no history buffer, as FR-251-AC-35 plans an unbounded operator; neither infinite-trace plan is refused `NotTimedClause`.
