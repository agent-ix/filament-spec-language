---
id: TC-2456
title: "The fsl binary writes the text outcome by default with the machine-mode exit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-2456: The fsl binary writes the text outcome by default with the machine-mode exit

## Description

Verify the driver's default text mode. Scope: FR-865-AC-3, FR-865-AC-4. Both are driver tests in the filament-driver repository.

## Test Procedure

1. Run `fsl check`, `fsl parse`, `fsl compile` and `fsl run` without `--format` and with `--format json`, each over a succeeding and a refused input.
2. Run `fsl run nosuch.json` and `fsl compile nosuch.json` without `--format`; run each verb with `--format text` and with `--format bogus`.

## Expected Results

- Step 1: the text output is the text form of the JSON document; the exit codes are equal.
- Step 2: the missing file refuses with its I/O code, exit 20, never 21; `--format text` is accepted; `--format bogus` refuses `invalid-request`, exit 20, naming `--format` and `bogus`, before any file is opened.

Test: filament-driver `fsl-cli/tests/cli_contract.rs` and `fsl-cli/tests/frontend_cli.rs`.
