---
id: TC-1711
title: "Items take the most severe exit and their own budgets"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: verifies
---
# TC-1711: Items take the most severe exit and their own budgets

## Description

Items take the most severe exit and their own budgets.

Scope: FR-651-AC-4.

## Test Procedure

Run FR-651-AC-4's violation plus zero-budget `seven` request, then the per-item budget request.

Tag the tests `#[trace("TC-1711", "FR-651-AC-4")]`; they live in `tests/it/run_items.rs`.

## Expected Results

Exit 22 for the first; each listed `seven` completes under the second.
