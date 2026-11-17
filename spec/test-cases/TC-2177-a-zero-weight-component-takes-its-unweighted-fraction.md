---
id: TC-2177
title: "A zero-weight component takes its unweighted fraction"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-199
    type: verifies
---
# TC-2177: A zero-weight component takes its unweighted fraction

## Description

A zero-weight component takes its unweighted fraction.

Scope: FR-199-AC-4, FR-199-AC-5.

## Test Procedure

Decide FR-199-AC-3's `Avail` variant whose only reachable cycle carries `duration` 0, then check its certificate with `tools/fsl-probability-check`. Test: `fsl-analyze/tests/it/weighted_long_run_certificate.rs` `long_run_zero_weight_component`, `zero_weight_certificate_checks`, and `reachable_zero_weight_components_take_their_unweighted_fraction` (negative: a document copied from the positive-weight variant is not accepted).

Tag the tests `#[trace("TC-2177", "FR-199-AC-4")]`.

## Expected Results

- Value `1800/1801`, settled `proved`; the independent checker accepts the certificate; no `ZeroWeightComponent`.
- A document copied from the positive-weight variant is not accepted.
