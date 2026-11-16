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

Produce the plans of the same clause under the fixed-sample profile and of the unbounded clause under the infinite-trace profile. Test: `fsl-analyze/tests/it/monitor_plan.rs` `fixed_sample_and_infinite_trace_clauses_are_planned`.

Tag the tests `#[trace("TC-2386", "FR-251-AC-53")]`.

## Expected Results

- Both plans are produced; the fixed-sample one has interval `[0, 3]` and capacity 5.
