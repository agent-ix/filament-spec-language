---
id: TC-2148
title: "Two independent syntax errors give two diagnostics"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2148: Two independent syntax errors give two diagnostics

## Description

Two independent syntax errors give two diagnostics.

Scope: FR-762-AC-1.

## Test Procedure

Parse a unit of three `record` declarations, the first missing a `;` between two fields and the third missing a field type, with `fsl_cst::parse` under the default limits. Test: `fsl-cst/tests/it/complete_cst.rs` `two_independent_syntax_errors_give_two_diagnostics`.

Tag the tests `#[trace("TC-2148", "FR-762-AC-1")]`.

## Expected Results

- The `ParsedSource` is not admissible.
- Exactly two `invalid_syntax`/`unexpected-token` diagnostics, in source order, at the first and third records' offending tokens.
