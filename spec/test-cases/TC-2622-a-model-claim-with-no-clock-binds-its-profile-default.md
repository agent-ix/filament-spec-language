---
id: TC-2622
title: "A model claim with no clock member checks under its profile's default"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-942
    type: verifies
---
# TC-2622: A model claim with no clock member checks under its profile's default

## Description

Verify the profile-chosen default binding and the refusal of a timed-profile
claim over an untimed model. Scope: FR-942-AC-1, FR-942-AC-2.

## Test Procedure

1. Over the timed ConfigVersion model under the event-position profile, check `eventually[0,3] holds(c.versionNumber = 2)` with no `clock`, the same with `clock "model-steps"`, and `eventually[0 ms, 3 ms] holds(c.versionNumber = 2)` with no `clock`.
2. Over the untimed model, check `eventually[0 ms, 3 ms] holds(c.versionNumber = 2)` with no `clock` under the timed profile, and `eventually holds(c.versionNumber = 2)` under infinite-trace.

Test: `fsl-semantics/tests/it/temporal_clock_names.rs`.

## Expected Results

- Step 1: `ModelSteps` with equal keys; the third refuses `ill_typed`/`type-mismatch` at the interval; no `MissingClock`.
- Step 2: `invalid_timed_model`/`model-time-on-untimed-model` at the `using` alias naming `Config`; `ModelSteps`.
