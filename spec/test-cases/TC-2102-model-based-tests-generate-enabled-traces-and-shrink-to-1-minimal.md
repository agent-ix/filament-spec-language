---
id: TC-2102
title: "Model-based tests generate enabled traces and shrink to a 1-minimal counterexample"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: verifies
---
# TC-2102: Model-based tests generate enabled traces and shrink to a 1-minimal counterexample

## Description

Verify generation, the native oracle and the shrinking guarantee. Scope: FR-752-AC-1 to FR-752-AC-3. Tests live in `fsl-eval/tests/it/model_based_tests.rs`.

## Test Procedure

1. Run model-based testing over `test/counters` with `inc` on `Int[0, 5]` and `value <= 3`.
2. For each seed of a fixed set, record every accepted shrink step and test the retained case for 1-minimality.
3. Run the `verify` step `check t claim Bounded method model_based_test domain M::Counter bound 50;` over a passing clause.

## Expected Results

- Step 1: only enabled invocations; boundary cases first; retained counterexample four `inc` from 0, replay-reproduced.
- Step 2: every step strictly smaller, domain-valid, enabled and failing the same clause; no single reduction still fails.
- Step 3: at most 50 cases, the record names the step, `tested` evidence and no verdict.
