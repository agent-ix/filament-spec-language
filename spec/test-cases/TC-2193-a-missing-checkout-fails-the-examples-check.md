---
id: TC-2193
title: "A missing checkout fails the examples check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2193: A missing checkout fails the examples check

## Description

A missing driver or examples checkout is an error, never a skip.

Scope: FR-765-AC-4.

## Test Procedure

Run `make examples-check` with the built xtask binary and, in turn: an absent
`FSL_DRIVER_DIR`; a stand-in driver folder with an absent `FSL_EXAMPLES_DIR`;
a stand-in driver with an examples folder that has no
`tools/fsl-examples/Cargo.toml`; an empty `FSL_DRIVER_DIR`.

Tag the tests `#[trace("FR-765-AC-4", "TC-2193")]`.

## Expected Results

- Each run exits nonzero.
- Its error names the variable, the folder or that it is not set, and the
  missing file, and no later recipe line runs.
