---
id: TC-2379
title: "Source and compile diagnostics carry null details"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-2379: Source and compile diagnostics carry null details

## Description

Source and compile diagnostics carry null details.

Scope: FR-286-AC-19.

## Test Procedure

Build the `execute` document of a run whose unit S3 refuses `ill_typed` and the `parse` document of FR-763-AC-2's unit, and read every diagnostic's `details`. Test: `tests/it/outcome_diagnostic_details.rs` `source_and_compile_diagnostics_carry_null_details`.

Tag the tests `#[trace("TC-2379", "FR-286-AC-19")]`.

## Expected Results

- Each `details` is `null`; no diagnostic holds a member outside the closed vocabulary or an empty object.
