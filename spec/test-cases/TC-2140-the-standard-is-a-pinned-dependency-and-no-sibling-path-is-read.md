---
id: TC-2140
title: "The standard is a pinned Cargo dependency and no sibling path is read"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: verifies
---
# TC-2140: The standard is a pinned Cargo dependency and no sibling path is read

## Description

The standard is a pinned Cargo dependency and no sibling path is read.

Scope: FR-760-AC-1, FR-760-AC-2.

## Test Procedure

Run `cargo metadata --locked --offline --format-version 1` in the workspace and find `filament-specification-qualification`. Read `proposals/checked-package-v2/fixtures/positive-control-operations.json` from that package's directory. Scan the `Makefile`, `.github/workflows`, `xtask/src`, `src`, `tests` and every `fsl-*/src` and `fsl-*/tests` for `FSPEC_DIR` and for a sibling `filament-specification` path, and read every workflow step for a checkout of `agent-ix/filament-specification`.

Tag the tests `#[trace("TC-2140", "FR-760-AC-1", "FR-760-AC-2")]`.

## Expected Results

- The package's source is `git+https://github.com/agent-ix/filament-specification?tag=v...`.
- The fixture's `contract_version` equals `fsl_package::CHECKED_PACKAGE_V2`.
- No scanned file reads or sets `FSPEC_DIR` or names a sibling standard path.
- No workflow step checks out `agent-ix/filament-specification`.
