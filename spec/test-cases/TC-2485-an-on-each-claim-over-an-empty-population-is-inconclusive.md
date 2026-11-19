---
id: TC-2485
title: "An on-each claim over an empty population is inconclusive"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: verifies
---
# TC-2485: An on-each claim over an empty population is inconclusive

## Description

An on-each claim over an empty population is inconclusive.

Scope: FR-875-AC-1.

## Test Procedure

Run `fsl analyze` over the `Link` model with the on-each claim `E` and a start snapshot whose `msgs` population holds no object. Test: `fsl-analyze/tests/it/probability_from_source.rs` `probabilistic_on_each_empty_population`.

Tag the tests `#[trace("TC-2485", "FR-875-AC-1")]`.

## Expected Results

- The item settles `inconclusive` with cause `no-instance`; it is neither `proved` nor `unsupported`.
