---
id: TC-2491
title: "Strict-clock verdicts carry region certificates"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-922
    type: verifies
---
# TC-2491: Strict-clock verdicts carry region certificates

## Description

Strict-clock verdicts carry region certificates.

Scope: FR-922-AC-1, FR-922-AC-2, FR-922-AC-3, FR-922-AC-4.

## Test Procedure

Prove `Deadline`, `Max` and `MeanTime` over `Retx` with the guard `self.x > 1 ms` under every scheduler; tamper the route and the values. Tests: `fsl-analyze/tests/it/timed_product_certificate.rs` `strict_guard_is_certified_over_regions`, `digital_values_for_a_strict_guard_are_rejected` (negative), `strict_mean_time_is_certified_over_boundary_regions`. For AC-4, `timed_product_certificate/region_successor.rs`: `a_delay_reaches_the_region_between_two_clocks_integer_crossings`, `a_strict_invariant_does_not_turn_the_skip_into_a_time_lock`, `a_closed_invariant_keeps_its_last_instant`, `an_until_over_the_skipped_region_is_proved`.

Tag each test with `#[trace("TC-2491", ...)]` and the FR-922 criterion it asserts.

## Expected Results

- `Deadline` `ExactValue{99/100}` and `Max` `ExactValue{999/1000}`, region documents accepted.
- A digital route is refused `ObjectiveMismatch`; values with initial `9999/10000` are rejected `InequalityFails`.
- `MeanTime`'s maximum `20/9 ms` has an accepted boundary-region document.
- AC-4's two-clock subjects settle as stated there.
