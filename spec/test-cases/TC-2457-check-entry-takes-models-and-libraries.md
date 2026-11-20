---
id: TC-2457
title: "check_outcome checks a unit against the models and libraries it imports"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: verifies
---
# TC-2457: check_outcome checks a unit against the models and libraries it imports

## Description

Verify `fsl_command::command::check_outcome` and the `missing_import` message. Scope: FR-866-AC-1 to FR-866-AC-4 and FR-866-AC-7.

## Test Procedure

1. Call `check_outcome` over FR-108's ConfigVersion unit with the model selection, then over the unit with an ill-typed invariant, beside a compile request over the same files.
2. Call it over FR-027-AC-10's program importing `test/geometry` with and without the library.
3. Compile FR-108's unit with `models` `[]`; call `check_outcome` with no model selection.
4. Call it with an unselected model identity, with a model over a unit with no `model` line, and with an unimported library.
5. Call it over the `Eligible` source with `dependent_bytes` 0 and no selection; over FR-027-AC-10's program with its `test/geometry` library of `N` bytes at `dependent_bytes` `N`, again with the checked source padded past `N` bytes, and at `N - 1`.

## Expected Results

- Step 1: success at S4; the ill-typed refusal equals the compile's in code, cause, locus and stage.
- Step 2: success with the library; the compile's `missing_import` refusal without it.
- Step 3: both refuse `missing_import`/`missing-selection` at I1 with a message naming `example/config-version`, `--model` and `models`.
- Step 4: `invalid-request` naming `--model` or `--library` and the identity; no model file is read.
- Step 5: the `Eligible` check succeeds at S4 with no diagnostic; both checks at `N` succeed at S4; at `N - 1` the document is `incomplete`, `last_stage` `null`, one `resource_exhausted` diagnostic naming `dependent_bytes` and `N - 1`, never success.

Test: `tests/it/check_imports.rs`.
