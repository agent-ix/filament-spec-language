---
id: TC-2152
title: "A unit with one syntax error keeps its one diagnostic"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2152: A unit with one syntax error keeps its one diagnostic

## Description

A unit with one syntax error keeps its one diagnostic.

Scope: FR-762-AC-5.

## Test Procedure

Parse each FR-002-AC-2 unit (`a < b < c`, `a = b != c`, `a < b + c = d`). Test: `fsl-cst/tests/it/complete_cst.rs` `a_single_error_keeps_its_one_diagnostic`.

Tag the tests `#[trace("TC-2152", "FR-762-AC-5")]`.

## Expected Results

- Each gives exactly one `invalid_syntax` diagnostic at the second comparison operator.
