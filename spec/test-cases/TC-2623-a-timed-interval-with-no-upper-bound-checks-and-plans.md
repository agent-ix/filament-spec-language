---
id: TC-2623
title: "A lower-bounded interval with no upper bound checks and is planned"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-943
    type: verifies
---
# TC-2623: A lower-bounded interval with no upper bound checks and is planned

## Description

Verify timed `[q, *]` checking, its monitor plan, the step `[a, *]` plan under
infinite-trace, and entry generation for unbounded past operators. Scope:
FR-943-AC-1 to FR-943-AC-4.

## Test Procedure

1. Under the timed profile over the timed ConfigVersion model, check `once[5 ms, *]`, `eventually[5 ms, *]`, `once(5 ms, *)` and `once[5, *]`.
2. Plan FR-251-AC-34's `once[5 ms, *] holds(p)` and evaluate it at ticks 4000 and 6000.
3. Under infinite-trace, plan `once[3,*] holds(p)` and `eventually[2,*] holds(p)` and evaluate them.
4. Generate entries for `once holds(p)` and `historically holds(p)` under FR-251-AC-22's premises.

Test: `fsl-analyze/tests/it/open_intervals.rs`.

## Expected Results

- Step 1: the first three check with no upper bound; `once[5, *]` refuses `ill_typed`/`type-mismatch`; none refuses `OpenInterval`.
- Step 2: retention 5001, capacity 12; `indeterminate`, then `true`.
- Step 3: retention 4, capacity 5; `indeterminate` at 2 and `true` at 3; `eventually` discharged at position 2, not 1.
- Step 4: entries generated, no `UnboundedPast` refusal.
