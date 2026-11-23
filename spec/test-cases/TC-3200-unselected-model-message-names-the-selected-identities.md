---
id: TC-3200
title: "An unselected --model identity names the identities the source selects"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-866
    type: verifies
---
# TC-3200: An unselected --model identity names the identities the source selects

## Description

Verify the message of a --model identity that no model declaration selects. Scope: FR-866-AC-8 and FR-866-AC-9.

## Test Procedure

1. Call `check_outcome` over FR-108's unit extended with `model Other = "example/other"` with the model selection `example:shop/model`, in text and machine mode.
2. Call it over `tests/fixtures/spine-compile.native` with the same selection.

## Expected Results

- Step 1: `invalid-request`, exit 20, message exactly `invalid clause request member --model: --model example:shop/model is not selected by the source; the source selects: example/config-version, example/other` in both modes.
- Step 2: message exactly `invalid clause request member --model: --model example:shop/model is not selected by the source; the source selects no models`.

Test: `fsl-command/tests/it` (check_outcome selections).
