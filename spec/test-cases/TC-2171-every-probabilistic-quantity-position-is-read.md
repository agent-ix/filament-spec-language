---
id: TC-2171
title: "Every probabilistic quantity position is read"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: verifies
---
# TC-2171: Every probabilistic quantity position is read

## Description

Every probabilistic quantity position is read.

Scope: FR-186-AC-14.

## Test Procedure

For each quantity row of FR-186 "Quantity positions" over the service model (and `Retx` for time rows), check the position with a quantity of its dimension and with `5 m`; check `fraction holds(P) over [0, 10] <= 5 ms` and `steps from holds(A) until holds(B) within 5 >= 5 ms`. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `quantity_literal_positions_check_and_refuse_by_dimension`.

Tag the tests `#[trace("TC-2171", "FR-186-AC-14")]`.

## Expected Results

- Right dimension: checks. `5 m`: `ill_typed`/`unit-mismatch` naming `Length` at the literal.
- Fraction and `steps` thresholds: `ill_typed`/`type-mismatch` at `5 ms`; no `not-yet-implemented`.
