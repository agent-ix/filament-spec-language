---
id: TC-2378
title: "Function and request diagnostics carry their details"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-2378: Function and request diagnostics carry their details

## Description

Function and request diagnostics carry their details.

Scope: FR-286-AC-18.

## Test Procedure

Build the `execute` outcome documents of `nope`, of `id_rec` with an argument naming `zz`, and of `id_big` with `"7"`. Test: `tests/it/outcome_diagnostic_details.rs` `function_and_request_diagnostics_carry_details`.

Tag the tests `#[trace("TC-2378", "FR-286-AC-18")]`.

## Expected Results

- `details` is `{"function": "nope"}`, `{"parameter": "zz"}` and `{"path": "/call/arguments/0/value"}`.
