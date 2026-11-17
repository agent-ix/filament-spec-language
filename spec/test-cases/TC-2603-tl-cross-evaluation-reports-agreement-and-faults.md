---
id: TC-2603
title: "TL cross-evaluation reports agreement and faults"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-2603: TL cross-evaluation reports agreement and faults

## Description

Verify the `cross_evaluation` record, the fault exit and the TL dependency
placement. Scope: FR-931-AC-4, FR-931-AC-5.

## Test Procedure

1. Run `Low` with `cross_evaluate` `{"initial": "spine-snapshot.json", "max_steps": 4}`.
2. At the `fsl-command` library entry, cross-evaluate `Low` with `p0` mapped to the negated leaf and render the record.
3. Read every FSL workspace crate manifest.

Test: `tests/it/request_surface_s1_16.rs` `tl_cross_evaluation_agrees_and_reports_faults`.

## Expected Results

- Step 1: `traces` >= 1, `positions` >= `traces`, `agree` true, `faults` `[]`, exit 0.
- Step 2: one `correspondence-loss` fault naming `Low`, the trace index, position and both verdicts; `agree` false; exit 30.
- Step 3: only `fsl-command` names `tl-syntax` or `tl-mltl`.
