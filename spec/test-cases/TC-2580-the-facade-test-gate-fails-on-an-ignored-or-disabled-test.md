---
id: TC-2580
title: "The facade test gate fails on an ignored or disabled test"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-912
    type: verifies
---
# TC-2580: The facade test gate fails on an ignored or disabled test

## Description

The facade test gate fails on an ignored or disabled test.

Scope: FR-912-AC-3.

## Test Procedure

Add `#[ignore]` to the `cf_pr_03_*` test of the TC-2578 directory and run the gate; then replace it with `#[cfg(any())]` and run it again.

Tag the tests `#[trace("TC-2580", "FR-912-AC-3")]`.

## Expected Results

- Each run exits 1.
- Each report names the function, its file and `CF-PR-03`.
