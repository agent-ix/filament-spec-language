---
id: TC-2154
title: "A recovered unit is never admitted"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-762
    type: verifies
---
# TC-2154: A recovered unit is never admitted

## Description

A recovered unit is never admitted.

Scope: FR-762-AC-7.

## Test Procedure

Parse TC-2148's unit with `fsl_cst::parse`. Test: `fsl-cst/tests/it/complete_cst.rs` `a_recovered_unit_is_never_admitted`.

Tag the tests `#[trace("TC-2154", "FR-762-AC-7")]`.

## Expected Results

- The `ParsedSource` is not admissible.
- It holds both diagnostics; the FR-002 `parse` operation refuses it at S1 (exit 20).
