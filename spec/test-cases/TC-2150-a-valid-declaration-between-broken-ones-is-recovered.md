---
id: TC-2150
title: "A valid declaration between two broken ones is in the recovered CST"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2150: A valid declaration between two broken ones is in the recovered CST

## Description

A valid declaration between two broken ones is in the recovered CST.

Scope: FR-762-AC-3.

## Test Procedure

Parse TC-2148's unit and find the `Declaration` nodes of the recovered CST. Test: `fsl-cst/tests/it/complete_cst.rs` `a_valid_declaration_between_broken_ones_is_recovered`.

Tag the tests `#[trace("TC-2150", "FR-762-AC-3")]`.

## Expected Results

- A `Declaration` node spans exactly the second record's text.
- The CST renders the unit's exact bytes.
