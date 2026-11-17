---
id: TC-2149
title: "Two invalid tokens give two invalid-token diagnostics"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2149: Two invalid tokens give two invalid-token diagnostics

## Description

Two invalid tokens give two invalid-token diagnostics.

Scope: FR-762-AC-2.

## Test Procedure

Parse a unit with `$` in one declaration and `$` in a later one. Test: `fsl-cst/tests/it/complete_cst.rs` `two_invalid_tokens_give_two_invalid_token_diagnostics`.

Tag the tests `#[trace("TC-2149", "FR-762-AC-2")]`.

## Expected Results

- Exactly two `invalid_syntax`/`invalid-token` diagnostics, each at its own `$`.
- No `unexpected-token` diagnostic.
