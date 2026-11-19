---
id: TC-2581
title: "The facade test gate fails on a test for an unlisted fact"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-912
    type: verifies
---
# TC-2581: The facade test gate fails on a test for an unlisted fact

## Description

The facade test gate fails on a test for an unlisted fact.

Scope: FR-912-AC-4.

## Test Procedure

Add a live `cf_pr_99_unknown` test to the TC-2578 directory and run the gate. Read the `make ci` target's recipe.

Tag the tests `#[trace("TC-2581", "FR-912-AC-4")]`.

## Expected Results

- Exit 1, naming `cf_pr_99_unknown` and `CF-PR-99` as naming no inventory fact.
- `make ci` runs `cargo xtask facade-tests`.
