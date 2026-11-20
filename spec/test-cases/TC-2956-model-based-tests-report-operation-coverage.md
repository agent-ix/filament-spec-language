---
id: TC-2956
title: "Model-based tests report operation coverage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: verifies
---
# TC-2956: Model-based tests report operation coverage

## Description

Verify that `fsl-eval`'s model-based testing reports one coverage entry per
model operation, with an unreachable operation uncovered and its reason, and
that coverage changes no verdict. Scope: FR-752-AC-4 and FR-752-AC-6.

Catches a generator that reports coverage from the generated cases alone,
one that calls an operation beyond the trace bound unreachable, and one
that turns an uncovered operation into a failure.

## Test Procedure

1. Over the FR-752-AC-4 counter (`inc`, `reset`, `jam`), run model-based
   testing with seed 7, `max_cases` 100 and `max_trace_length` 8 over a
   clause every case passes.
2. Repeat with `max_trace_length` 3.
3. Repeat step 1 and compare `operations`.
4. Over FR-752-AC-6's counter (`value: Integer`, `add(by: Int[1, 100])`, `big()` enabled when `value >= 300`), run with seed 7, `max_cases` 100, `max_trace_length` 8 and `max_value_bits` 8, then with `max_value_bits` 10.

## Expected Results

- Step 1: `inc` and `reset` `covered` with their first case index; `jam`
  `uncovered`, reason `unreachable`; `coverage` `partial`; `tested`, no
  verdict.
- Step 2: `reset` `uncovered`, reason `unreachable-within-bound`, `bounds` `["max_trace_length"]`.
- Step 3: equal `operations`.
- Step 4: with 8, `big` `uncovered`, reason `unreachable-within-bound`, `bounds` `["max_value_bits"]`, never `unreachable`; `value_bound_cuts.count` at least 1; `tested`. With 10, `big` `covered` or `not-generated` with an `enabled_at` prefix summing to at least 300.
