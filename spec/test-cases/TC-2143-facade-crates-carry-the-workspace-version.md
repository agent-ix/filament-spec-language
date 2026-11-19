---
id: TC-2143
title: "Facade crates carry the workspace version"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-760
    type: verifies
---
# TC-2143: Facade crates carry the workspace version

## Description

Facade crates carry the workspace version.

Scope: FR-760-AC-5.

## Test Procedure

Run `cargo metadata --locked --offline --no-deps --format-version 1` and read the versions of `filament-spec-language`, `fsl-package`, `fsl-replay` and `fsl-analyze`.

Tag the tests `#[trace("TC-2143", "FR-760-AC-5")]`.

## Expected Results

- The three facade crates report the root package's version.
