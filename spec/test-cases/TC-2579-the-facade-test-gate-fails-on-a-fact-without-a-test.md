---
id: TC-2579
title: "The facade test gate fails on a fact without a test"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-912
    type: verifies
---
# TC-2579: The facade test gate fails on a fact without a test

## Description

The facade test gate fails on a fact without a test.

Scope: FR-912-AC-2.

## Test Procedure

Remove the `cf_pr_03_*` test from the TC-2578 directory and run the gate.

Tag the tests `#[trace("TC-2579", "FR-912-AC-2")]`.

## Expected Results

- Exit 1.
- The report names `CF-PR-03` as having no live test.
