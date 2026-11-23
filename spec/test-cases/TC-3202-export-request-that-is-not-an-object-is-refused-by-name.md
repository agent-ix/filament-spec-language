---
id: TC-3202
title: "An export request that is not an object is refused naming /request"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1047
    type: verifies
---
# TC-3202: An export request that is not an object is refused naming /request

## Description

Verify the export entry's refusal of a non-object request. Scope: FR-1047-AC-9.

## Test Procedure

1. Call `export_with_limits_and_overrides` with `ExportSelection::Ordinary` over `{"format":"native-run/1","request":3}`.

## Expected Results

- Step 1: `invalid-request`, stage `request`, exit 20, message exactly `invalid clause request member /request: expected an object`.

Test: `fsl-command/tests/it` (request shape).
