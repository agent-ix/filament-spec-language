---
id: TC-2371
title: "A Quantity round-trips with its unit name and normalized magnitude"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2371: A Quantity round-trips with its unit name and normalized magnitude

## Description

A Quantity round-trips with its unit name and normalized magnitude.

Scope: FR-650-AC-10.

## Test Procedure

Call `id_q` with a Decimal magnitude and with the unit `s`; call `id_bq` with magnitudes `10` and `11`. Test: `tests/it/run_value_spellings.rs` `a_quantity_round_trips_normalized`.

Tag the tests `#[trace("TC-2371", "FR-650-AC-10")]`.

## Expected Results

- `id_q` completes with unit `ms` and the Rational magnitude `3/2`; unit `s` refuses `wrong-value-kind` at `/unit`.
- `id_bq` completes for `10` and refuses `invalid-value` at `/magnitude` for `11`.
