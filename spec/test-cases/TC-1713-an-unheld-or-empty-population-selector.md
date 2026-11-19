---
id: TC-1713
title: "An unheld or empty population selector"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: verifies
---
# TC-1713: An unheld or empty population selector

## Description

An unheld or empty population selector.

Scope: FR-651-AC-6.

## Test Procedure

Run FR-651-AC-6's unheld-population and empty-population selectors.

Tag the tests `#[trace("TC-1713", "FR-651-AC-6")]`; they live in `tests/it/run_items.rs`.

## Expected Results

One `wrong-role-mapping` item and the other items run; zero items with `expanded` 0 and `category` `success`, exit 0.
