---
id: TC-1708
title: "An items request runs two clause selections"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: verifies
---
# TC-1708: An items request runs two clause selections

## Description

An items request runs two clause selections.

Scope: FR-651-AC-1.

## Test Procedure

Run FR-651-AC-1's `items` request with two `ParentOrder` selections.

Tag the tests `#[trace("TC-1708", "FR-651-AC-1")]`; they live in `tests/it/run_items.rs`.

## Expected Results

One `native-run-items/1` document with two items equal to the single runs, `category` `violation`, exit 10.
