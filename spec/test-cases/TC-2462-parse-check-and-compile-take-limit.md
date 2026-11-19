---
id: TC-2462
title: "fsl parse, check and compile take --limit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-869
    type: verifies
---
# TC-2462: fsl parse, check and compile take --limit

## Description

Verify the driver's `--limit` on `parse`, `check` and `compile`. Scope: FR-869-AC-3, FR-869-AC-4, driver tests in the filament-driver repository.

## Test Procedure

1. Run `fsl parse`, `fsl check` and `fsl compile` over a 6,000-deep nest with `--limit s1.nodes=1000000`, and `fsl parse` without it.
2. Run each with a setting it does not accept, an unknown setting, a malformed value and a repeated name.

## Expected Results

- Step 1: exit 0 with the option; exit 22 naming `s1.nodes` without it.
- Step 2: `invalid-request`, exit 20, naming the setting and the accepted settings, or FR-255 Behavior 6's usage refusal naming the operand; no file is read.

Test: filament-driver `fsl-cli/tests/cli_contract.rs`.
