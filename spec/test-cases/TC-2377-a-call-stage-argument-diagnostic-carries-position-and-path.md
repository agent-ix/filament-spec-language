---
id: TC-2377
title: "A call-stage argument diagnostic carries position and path"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-2377: A call-stage argument diagnostic carries position and path

## Description

A call-stage argument diagnostic carries position and path.

Scope: FR-286-AC-17.

## Test Procedure

Build the `execute` outcome documents of `id_rec` with `fields` lacking `y` and of `id_set` with a repeated element, and read the command-error envelope of each request. Test: `tests/it/outcome_diagnostic_details.rs` `argument_diagnostics_carry_position_and_path`.

Tag the tests `#[trace("TC-2377", "FR-286-AC-17")]`.

## Expected Results

- The diagnostics carry `details` `{"path": "/record/fields", "position": 0}` and `{"path": "/set/1", "position": 0}`.
- Each equals the envelope's `details`.
