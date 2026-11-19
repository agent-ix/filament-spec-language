---
id: TC-1710
title: "One refused item leaves the other items unchanged"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: verifies
---
# TC-1710: One refused item leaves the other items unchanged

## Description

One refused item leaves the other items unchanged.

Scope: FR-651-AC-3.

## Test Procedure

Run FR-651-AC-3's three `call` selections.

Tag the tests `#[trace("TC-1710", "FR-651-AC-3")]`; they live in `tests/it/run_items.rs`.

## Expected Results

Completed 4, the `missing-member` command-error envelope, completed 7; `category` `refusal`, exit 20.
