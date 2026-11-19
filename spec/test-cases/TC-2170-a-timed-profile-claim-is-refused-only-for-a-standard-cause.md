---
id: TC-2170
title: "A timed-profile claim is refused only for a standard cause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: verifies
---
# TC-2170: A timed-profile claim is refused only for a standard cause

## Description

A timed-profile claim is refused only for a standard cause.

Scope: FR-186-AC-13.

## Test Procedure

Check a timed-profile claim over the untimed service model, then the timed deadline claim over `Retx`. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `probabilistic_claim_timed_profile_untimed_model` and `fsl-analyze/tests/it/digital_clocks.rs` (the existing `Retx` claims check under the timed profile).

Tag the tests `#[trace("TC-2170", "FR-186-AC-13")]`.

## Expected Results

- Untimed: `invalid_timed_model`/`model-time-on-untimed-model` naming `Service` at the alias; no `timed-event-profile` cause.
- `Retx`: checks.
