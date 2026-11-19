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

Scope: FR-186-AC-12, FR-186-AC-15 and FR-186-AC-16 (timed halves).

## Test Procedure

Check the three forms of FR-186-AC-12 over `Retx`. Test: `fsl-analyze/tests/it/digital_clocks.rs` `time_forms_over_a_timed_model_check`.

Check FR-186-AC-15's two `weighted by delay` claims over `Retx`, and the same long-run claim unweighted, and compare the two long-run normal forms. Check FR-186-AC-16's elapsed quantile over `Retx` with the thresholds `3 ms`, `0.003 s` and the bare `3`.
Tag the tests `#[trace("TC-2169", "FR-186-AC-12")]`, the delay-weighting test `#[trace("TC-2169", "FR-186-AC-15")]` and the elapsed-quantile test `#[trace("TC-2169", "FR-186-AC-16")]`.

## Expected Results

- Each checks; the threshold and horizons are recorded in milliseconds as written.
- Delay weighting: both claims check with `weight` `Some(Weighting::Delay)` and no `missing-name`; the weighted and unweighted long-run normal forms differ.
- Elapsed quantile: `Quantile{q: 1/2, Elapsed{within: 5}, AtMost(3)}` for `3 ms` and `0.003 s`; the bare `3` refused `ill_typed`/`type-mismatch` at `3`.
