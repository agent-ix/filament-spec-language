---
id: TC-2486
title: "An on-each instance that never activates holds vacuously"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: verifies
---
# TC-2486: An on-each instance that never activates holds vacuously

## Description

An on-each instance that never activates holds vacuously.

Scope: FR-875-AC-2.

## Test Procedure

Run `E` over a start snapshot holding `m` undelivered and `n` delivered, at `rational(9, 10)` and at `rational(99, 100)`. Test: `fsl-analyze/tests/it/probability_from_source.rs` `probabilistic_on_each_never_activating_instance_is_vacuous`.

Tag the tests `#[trace("TC-2486", "FR-875-AC-2")]`.

## Expected Results

- At `9/10` the item is `proved`.
- At `99/100` the item is `refuted`; the counterexample names `m`, never `n`.
