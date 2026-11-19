---
id: TC-2458
title: "fsl check takes --model and --library"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: verifies
---
# TC-2458: fsl check takes --model and --library

## Description

Verify the driver's `check` options. Scope: FR-866-AC-5, a driver test in the filament-driver repository.

## Test Procedure

1. Run `fsl check im.fsl --model example/config-version=m.json` over FR-866-AC-5's unit, and without `--model`.
2. Run it with the operand twice, with `--model m.json` and with `--model =m.json`.

## Expected Results

- Step 1: exit 0; without the option exit 20 `missing_import` naming the identity and `--model`.
- Step 2: each exits 20 `invalid-request` naming `--model`, before `m.json` is read.

Test: filament-driver `fsl-cli/tests/cli_contract.rs`.
