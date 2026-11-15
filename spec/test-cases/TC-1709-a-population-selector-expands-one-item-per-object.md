---
id: TC-1709
title: "A population selector expands one item per object"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: verifies
---
# TC-1709: A population selector expands one item per object

## Description

A population selector expands one item per object.

Scope: FR-651-AC-2.

## Test Procedure

Run FR-651-AC-2's `each` clause selection and `sameIdentity` call selection.

Tag the tests `#[trace("TC-1709", "FR-651-AC-2")]`; they live in `tests/it/run_items.rs`.

## Expected Results

Items in document order with their `object` references, each equal to the single run; `expanded` 2.
