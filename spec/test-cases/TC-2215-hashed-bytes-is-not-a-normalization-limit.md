---
id: TC-2215
title: "hashed_bytes is not a normalization limit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: verifies
---
# TC-2215: hashed_bytes is not a normalization limit

## Description

Verify that an analyze request naming `hashed_bytes` among its normalization
limits is refused as an unknown member instead of being accepted with no
effect, and that a real normalization limit beside it does not rescue it.
Scope: FR-620-AC-11.

## Test Procedure

Analyze the Counter request for `Bounded` with `limits.normalization` set to
`{"hashed_bytes": 1}`, to `{"hashed_bytes": 1, "work_units": 5000000}` and to
`{"work_units": 5000000}`. Test: `fsl-analyze/tests/it/analyze_request_wire.rs`
`hashed_bytes_is_not_a_normalization_limit`.

## Expected Results

- The first two: exit 20, no items, an `invalid-request` diagnostic naming
  `hashed_bytes`.
- The third: admitted, exit 0.
