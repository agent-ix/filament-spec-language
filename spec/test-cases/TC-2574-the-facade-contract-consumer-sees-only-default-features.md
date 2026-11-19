---
id: TC-2574
title: "The facade contract consumer sees only default features"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-911
    type: verifies
---
# TC-2574: The facade contract consumer sees only default features

## Description

The facade contract consumer sees only default features.

Scope: FR-911-AC-1.

## Test Procedure

Run `cargo metadata` for the workspace and read the `facade-contract-consumer` package and its resolved dependencies.

Tag the tests `#[trace("TC-2574", "FR-911-AC-1")]`.

## Expected Results

- The package is a workspace member at `tests/fixtures/facade-contract-consumer`.
- Its dependencies on `fsl-package`, `fsl-replay` and `fsl-analyze` enable no feature beyond the default.
- No dependency of the crate enables a `test-support` feature of an `fsl-*` crate.
