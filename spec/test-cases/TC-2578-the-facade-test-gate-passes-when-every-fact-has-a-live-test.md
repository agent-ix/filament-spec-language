---
id: TC-2578
title: "The facade test gate passes when every fact has a live test"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-912
    type: verifies
---
# TC-2578: The facade test gate passes when every fact has a live test

## Description

The facade test gate passes when every fact has a live test.

Scope: FR-912-AC-1.

## Test Procedure

Write a scratch test directory holding a live `cf_<code>_<nn>_*` test for every fact of the pinned inventory and run `cargo xtask facade-tests --tests <dir>`.

Tag the tests `#[trace("TC-2578", "FR-912-AC-1")]`.

## Expected Results

- Exit 0.
- The report lists every inventory fact id with its test name.
