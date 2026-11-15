---
id: TC-767
title: "The direction check keeps frontends, analyze and the plugin host out of the qualified core"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-284
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-280
    type: verifies
---
# TC-767: The direction check keeps frontends, analyze and the plugin host out of the qualified core

## Description

Verify the `cargo tree` direction check over the FSL core crates, with injected violations.

Scope: FR-284-AC-1, FR-280-AC-3.

## Test Procedure

1. Run the direction check over the FSL workspace.
2. Run it over a test copy of the workspace whose `fsl-eval` manifest adds `fsl-analyze`.
3. Run it over a test copy whose `fsl-replay` manifest adds an argument-parser crate.
4. Run it over a test copy whose `fsl-route` manifest adds a CG crate.

Tag the tests `#[trace("TC-767", "<AC id>")]`.

## Expected Results

- Step 1: it passes.
- Step 2: it fails naming `(fsl-eval, fsl-analyze)`.
- Step 3: it fails naming `(fsl-replay, <that crate>)`.
- Step 4: it fails naming `(fsl-route, <that crate>)`.
