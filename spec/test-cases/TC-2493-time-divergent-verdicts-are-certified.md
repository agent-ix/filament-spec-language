---
id: TC-2493
title: "Time-divergent verdicts are certified"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-924
    type: verifies
---
# TC-2493: Time-divergent verdicts are certified

## Description

Time-divergent verdicts are certified.

Scope: FR-924-AC-1, FR-924-AC-2, FR-924-AC-3, FR-924-AC-4, FR-924-AC-5, FR-924-AC-6.

## Test Procedure

Prove `Deadline` over `Retx` with `toggle`, `MeanLow` over `Retx`, `Deadline` over the unguarded variant and `UnderMax` over the time-locked variant; tamper each as the criteria state. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `zero_time_components_take_their_best_exit`, `zeno_values_are_rejected` (negative), `delay_cycles_are_not_collapsed` (negative), `zero_time_cycle_without_end_component_is_certified`, `time_locked_state_is_certified_terminal`, `reset_loop_zeno_values_are_rejected` (negative).

Tag each test with `#[trace("TC-2493", ...)]` and the FR-924 criterion it asserts.

## Expected Results

- Toggle: `ExactValue{99/100}`, one value per pair; Zeno values rejected `InequalityFails`.
- `MeanLow` `10/9 ms`; initial `1 ms` rejected `InequalityFails`.
- Unguarded `Deadline` `ExactValue{1}` accepted; time-locked `UnderMax` `ExactValue{0}` accepted, value 1 rejected.
- Reset loop on regions: `ExactValue{99/100}` with a region certificate; Zeno values rejected `InequalityFails`.
