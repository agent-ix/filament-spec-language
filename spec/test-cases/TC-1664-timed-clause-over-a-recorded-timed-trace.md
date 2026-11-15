---
id: TC-1664
title: "Timed clause over a recorded timed trace"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: verifies
---
# TC-1664: Timed clause over a recorded timed trace

## Description

Verify FR-642-AC-1 to AC-4: a timed clause over a record trace subject compiles with a `timed` clock, and runs pointwise over finite and lasso timed traces with exact rational stamps.

## Test Procedure

Use FR-642-AC-1's `Reading` unit and `Ack` clause with `clocks` `{"plant": {"kind": "timed", "unit": "ms"}}`. Run each trace FR-642-AC-2 to AC-4 lists through `command::run`.

## Expected Results

- The unit compiles; with `clock "model-time"` it refuses `invalid_timed_model`/`model-time-on-trace-subject`.
- Each trace reports the verdict FR-642 states; stamps `0` and `1/3` compare exactly.
