---
id: TC-2995
title: "fsl export hands its output path and evidence request to FSL"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: verifies
---
# TC-2995: fsl export hands its output path and evidence request to FSL

## Description

Verify that the driver's `fsl export` verb hands `--output` to FSL as the request override, writes FSL's `export` outcome document unchanged, raises export limits through `--limit`, and routes `--evidence`. Scope: FR-287-AC-14. The `fsl` binary owns option parsing, so the test lives in the filament-driver repository.

## Test Procedure

1. Run `fsl export req.json --output m.tla --format json` over FR-756-AC-1's counter request; run the request carrying `export.output` naming `m.tla` through FSL in a fresh directory.
2. Run `fsl export req.json --limit export.output_bytes=1 --format json`.
3. Run `fsl export --evidence e.json --format json`.
4. Run `fsl export --evidence e.json --output m.tla`, `fsl export req.json --evidence e.json` and `fsl compile req.json --evidence e.json`.

## Expected Results

- Step 1: exit 0; the two files are byte-identical; stdout is FSL's document with operation `export`.
- Step 2: exit 22 naming `export.output_bytes`.
- Step 3: FSL's evidence outcome document with operation `export`.
- Step 4: the first two refuse `invalid-request` naming `--evidence`, exit 20; the third refuses an unknown option, exit 20.
