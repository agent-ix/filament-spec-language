---
id: TC-2104
title: "A synthesis declaration builds, checks its bound and runs"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-754
    type: verifies
---
# TC-2104: A synthesis declaration builds, checks its bound and runs

## Description

Verify the `synthesis` declaration through S1, S2, S3 and the synthesizer. Scope: FR-754-AC-1 to FR-754-AC-3.

## Test Procedure

1. Build `Pick` at S2, with and without `minimize Pick`.
2. Check the three unbounded domains FR-754-AC-2 lists.
3. Run `Pick` with `Pick * Pick = 49`, `= 50`, and `minimize Pick` with `Pick >= 3`.

## Expected Results

- Step 1: one `Synthesis` form with its spans, and the objective direction and expression.
- Step 2: each refused `unsupported_projection`/`unbounded-extent` at the `domain` name naming the component.
- Step 3: `candidate` 7 `validated`; `no-candidate` after ten values; optimal 3.
