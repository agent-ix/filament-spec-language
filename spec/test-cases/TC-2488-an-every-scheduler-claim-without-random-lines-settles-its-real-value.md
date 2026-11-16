---
id: TC-2488
title: "An every-scheduler claim without random lines settles its real value"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-876
    type: verifies
---
# TC-2488: An every-scheduler claim without random lines settles its real value

## Description

An every-scheduler claim without random lines settles its real value.

Scope: FR-876-AC-1, FR-876-AC-2, FR-876-AC-3.

## Test Procedure

Compile and analyze the `Link` unit with its two `random` lines removed and the claim `Every`, with exact evidence; then the same claim over the unit with its `random` lines. Tests: `fsl-analyze/tests/it/probability_from_source.rs` `every_scheduler_without_random_compiles`, `every_scheduler_without_random_is_refuted` (negative) and `every_scheduler_with_random_is_proved`.

Tag each test with `#[trace("TC-2488", ...)]` and the FR-876 criterion it asserts.

## Expected Results

- Without `random` lines the unit compiles with exit 0 and no diagnostic, and the item settles `refuted` with value 0 and a reproduced counterexample.
- With the `random` lines the item settles `proved` with exact value `24/25`.
