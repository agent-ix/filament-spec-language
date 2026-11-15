---
id: TC-2141
title: "Pin check refuses a branch dependency"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: verifies
---
# TC-2141: Pin check refuses a branch dependency

## Description

Pin check refuses a branch dependency.

Scope: FR-760-AC-3.

## Test Procedure

Run `cargo xtask pin-check` in the workspace. Then run it with a scratch workspace whose manifest declares `filament-contract-model = { git = "https://github.com/agent-ix/filament-contract-ir", branch = "main" }`.

Tag the tests `#[trace("TC-2141", "FR-760-AC-3")]`.

## Expected Results

- The workspace passes, exit 0.
- The scratch workspace exits 1 and the report names its manifest and `filament-contract-model`.
