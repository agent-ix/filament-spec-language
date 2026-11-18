---
id: TC-2952
title: "Model-based tests report operation coverage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: verifies
---
# TC-2952: Model-based tests report operation coverage

## Description

Verify that `fsl-eval`'s model-based testing reports one coverage entry per
model operation, with an unreachable operation uncovered and its reason, and
that coverage changes no verdict. Scope: FR-752-AC-4.

Catches a generator that reports coverage from the generated cases alone,
one that calls an operation beyond the trace bound unreachable, and one
that turns an uncovered operation into a failure.

## Test Procedure

1. Over the FR-752-AC-4 counter (`inc`, `reset`, `jam`), run model-based
   testing with seed 7, `max_cases` 100 and `max_trace_length` 8 over a
   clause every case passes.
2. Repeat with `max_trace_length` 3.
3. Repeat step 1 and compare `operations`.

## Expected Results

- Step 1: `inc` and `reset` `covered` with their first case index; `jam`
  `uncovered`, reason `unreachable`; `coverage` `partial`; `tested`, no
  verdict.
- Step 2: `reset` `uncovered`, reason `unreachable-within-bound`.
- Step 3: equal `operations`.
