---
id: TC-2442
title: "The partial, completeness, activation and repair authority rules have one owner"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-861
    type: verifies
---
# TC-2442: The partial, completeness, activation and repair authority rules have one owner

## Description

Verify that FSL's temporal evaluator and `filament-observation`'s authority
modules agree on shared cases, that a disagreement fails a test, and that
FSL holds no implementation of the four rule sets of its own.
Scope: FR-861-AC-1, FR-861-AC-2, FR-861-AC-3, FR-861-AC-4.

## Test Procedure

1. Run FR-501-AC-13's counter-cover cases through the evaluator and the owner's `partial` module.
2. Run FR-502-AC-2's completeness revisions through the evaluator's ledger, recording the owner `completeness::check_revision` answer each one reads.
3. Run FR-524-AC-27 to AC-29's scopes through the owner's `activation` module.
4. Read `fsl-eval/Cargo.toml` and scan FSL's shipped source for functions with two parameters of type `ProgressCompleteness` or comparing two `ProgressCompleteness` values, uses of `filament_observation::authority::repair`, and items named for an affected-region plan.

## Expected Results

- Steps 1 to 3: equal answers on every case; a case with different answers fails naming the case and both answers.
- Step 2: each ledger acceptance and refusal is the `check_revision` answer, its typed cause read from it.
- Step 4: `fsl-eval` names `filament-observation` and calls its `partial`, `completeness` and `activation` modules; the progress ledger calls `completeness::check_revision`; the scan finds no such function, use or item, and names the file and item of any it finds.

Test: `fsl-analyze/tests/it/monitor_authority/` (steps 1 to 3) and `fsl-eval/tests/it/observation_owner.rs` (step 4).
