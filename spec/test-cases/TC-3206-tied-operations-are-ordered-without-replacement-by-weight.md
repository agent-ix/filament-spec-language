---
id: TC-3206
title: "Tied operations are ordered without replacement by weight"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: verifies
---
# TC-3206: Tied operations are ordered without replacement by weight

## Description

Three operations tie at one instant, each still enabled at its turn; the order after the first pick changes the checked value.

Scope: FR-204-AC-20.

## Test Procedure

Build `Chain` (FR-204-AC-20) under `Order` (`X` 1, `Y` 2, `Z` 3); decide the `last = X`, `last = Z` and `last = Y` (`<= 0.3`) claims; submit the `c.last = Z` `Exact` certificates computed with the first pick weighted and the rest canonical (`1/2`), with even orders (`1/3`) and with canonical order (`1`). Test: `fsl-analyze/tests/it/digital_clocks.rs` `tied_operations_are_ordered_without_replacement`.

Tag the tests `#[trace("TC-3206", "FR-204-AC-20")]`.

## Expected Results

- `last = X` `ExactValue{7/12}`, `last = Z` `ExactValue{3/20}`, `last = Y` `ExactValue{4/15}`, each claim `proved`.
- All three certificates rejected `InequalityFails`.
