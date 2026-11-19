---
id: TC-3029
title: "An export whose file cannot be written is an output failure"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3029: An export whose file cannot be written is an output failure

## Description

Verify that a write failure after a successful export serializes as an output failure: internal-failure, exit 30, the stage the export reached, and an `output-failure` diagnostic. Scope: FR-286-AC-23.

## Test Procedure

1. Export `SlowEnough` over `Braking` under `minimum_fidelity` `any` with `output` `{"file": "missing-directory/m.tla"}` in a directory with no `missing-directory`.
2. Serialize the run's outcome document for operation `export`.

## Expected Results

- Step 1: exit 30; the refusal envelope has `code` and `cause` `output-failure`; no file or directory is created.
- Step 2: `operation` `export`, `category` internal-failure, `last_stage` S4, `result` `null`, one diagnostic with `code` `output-failure`, `cause` `output-failure` and `details` `null`.
