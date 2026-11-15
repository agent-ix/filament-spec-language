---
id: TC-1715
title: "Malformed context and value-invariant selections refuse"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-652
    type: verifies
---
# TC-1715: Malformed context and value-invariant selections refuse

## Description

Malformed context and value-invariant selections refuse.

Scope: FR-652-AC-2.

## Test Procedure

Run each malformed selection FR-652-AC-2 lists.

Tag the tests `#[trace("TC-1715", "FR-652-AC-2")]`; they live in `tests/it/run_items.rs`.

## Expected Results

Each refuses `invalid-request` at stage `request`, exit 20, naming the member.
