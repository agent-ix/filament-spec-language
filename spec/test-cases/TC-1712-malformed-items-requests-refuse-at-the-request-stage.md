---
id: TC-1712
title: "Malformed items requests refuse at the request stage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: verifies
---
# TC-1712: Malformed items requests refuse at the request stage

## Description

Malformed items requests refuse at the request stage.

Scope: FR-651-AC-5.

## Test Procedure

Run each malformed request FR-651-AC-5 lists, and the syntax-error program.

Tag the tests `#[trace("TC-1712", "FR-651-AC-5")]`; they live in `tests/it/run_items.rs`.

## Expected Results

Each refuses `invalid-request` at stage `request`, exit 20, naming the member; the syntax error refuses at stage `source` with one envelope and no items document.
