---
id: TC-2492
title: "Every-scheduler delay races are certified"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-923
    type: verifies
---
# TC-2492: Every-scheduler delay races are certified

## Description

Every-scheduler delay races are certified.

Scope: FR-923-AC-1, FR-923-AC-2, FR-923-AC-3.

## Test Procedure

Prove `Deadline` over `Retx` with `delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };` under every scheduler and the `>= 0.995` claim; tamper its values to the distribution-free reading; decide the `send`/`tick` tie under every scheduler and under `Both`. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `every_scheduler_discrete_delay_is_certified`, `distribution_free_values_are_rejected` (negative), `every_scheduler_tie_is_the_schedulers_choice`.

Tag each test with `#[trace("TC-2492", ...)]` and the FR-923 criterion it asserts.

## Expected Results

- `ExactValue{159129/160000}` minimum and maximum, document accepted; at 0.995 `refuted`.
- Distribution-free values: `CertificateRejected`, `InequalityFails`.
- The `send`/`tick` tie: under every scheduler maximum 1 with `tie` `tick`, `<= 0.6` refuted; under `Both` `21/40`, proved.
