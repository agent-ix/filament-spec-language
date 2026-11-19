---
id: TC-2389
title: "The timed profile refuses a model-steps clock as a contradiction"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: verifies
---
# TC-2389: The timed profile refuses a model-steps clock as a contradiction

## Description

The timed profile refuses a model-steps clock as a contradiction.

Scope: FR-233-AC-5.

## Test Procedure

Check the `Rpc` clause under the timed profile with `clock "model-steps"`, under event-position with it, and under the timed profile with no clock. Test: `fsl-semantics/tests/it/temporal_clock_names.rs` `the_timed_profile_refuses_model_steps`.

Tag the tests `#[trace("TC-2389", "FR-233-AC-5")]`.

## Expected Results

- The first refuses `ill_typed`/`type-mismatch` at the clock string; the others check as `ModelSteps` and `ModelTime`.
