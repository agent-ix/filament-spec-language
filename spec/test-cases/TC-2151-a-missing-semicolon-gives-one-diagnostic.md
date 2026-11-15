---
id: TC-2151
title: "A missing semicolon gives exactly one diagnostic"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2151: A missing semicolon gives exactly one diagnostic

## Description

A missing semicolon gives exactly one diagnostic.

Scope: FR-762-AC-4.

## Test Procedure

Parse a unit of two `record` declarations whose first lacks its final `;`. Test: `fsl-cst/tests/it/complete_cst.rs` `a_missing_semicolon_gives_one_diagnostic`.

Tag the tests `#[trace("TC-2151", "FR-762-AC-4")]`.

## Expected Results

- Exactly one diagnostic.
- The second record's `Declaration` node is present.
