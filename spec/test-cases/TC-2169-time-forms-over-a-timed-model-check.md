---
id: TC-2169
title: "Time forms over a timed model check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: verifies
---
# TC-2169: Time forms over a timed model check

## Description

Time forms over a timed model check.

Scope: FR-186-AC-12, FR-186-AC-15 (timed half).

## Test Procedure

Check the three forms of FR-186-AC-12 over `Retx`. Test: `fsl-analyze/tests/it/digital_clocks.rs` `time_forms_over_a_timed_model_check`.

Check FR-186-AC-15's two `weighted by delay` claims over `Retx`, and the same long-run claim unweighted, and compare the two long-run normal forms.
Tag the tests `#[trace("TC-2169", "FR-186-AC-12")]`, and the delay-weighting test `#[trace("TC-2169", "FR-186-AC-15")]`.

## Expected Results

- Each checks; the threshold and horizons are recorded in milliseconds as written.
- Delay weighting: both claims check with `weight` `Some(Weighting::Delay)` and no `missing-name`; the weighted and unweighted long-run normal forms differ.
