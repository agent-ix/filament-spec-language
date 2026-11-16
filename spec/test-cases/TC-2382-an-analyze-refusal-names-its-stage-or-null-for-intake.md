---
id: TC-2382
title: "An analyze refusal names its stage, or null for intake"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-2382: An analyze refusal names its stage, or null for intake

## Description

An analyze refusal names its stage, or null for intake.

Scope: FR-621-AC-12.

## Test Procedure

Analyze a malformed request, a checker-refused program, a snapshot naming an undeclared population, and a request whose cancel is already tripped. Test: `fsl-analyze/tests/it/analyze_last_stage.rs` `refusals_name_their_stage`.

Tag the tests `#[trace("TC-2382", "FR-621-AC-12")]`.

## Expected Results

- `last_stage` is `null`, `S3`, `S4` and `null` respectively.
