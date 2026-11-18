---
id: TC-2994
title: "An export outcome document carries operation export"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-2994: An export outcome document carries operation export

## Description

Verify the `export` operation's outcome document: operation, category per export status, stage, empty items and artifacts, and the `exported` result. Scope: FR-286-AC-22.

## Test Procedure

1. Serialize the outcome of FR-756-AC-1's counter export.
2. Serialize the outcome of FR-1015-AC-1's below-floor export.
3. Serialize the outcome of the counter export under `export.output_bytes` 1.
4. Serialize the outcome of the counter export naming an existing output path.

## Expected Results

- Step 1: `operation` `export`, success, `last_stage` S4, `items` `[]`, `artifacts` `[]`, `result` `{"kind": "exported", "export": E}` with `E` the run outcome's `export` member.
- Step 2: unsupported, `result` kind `exported`.
- Step 3: incomplete, `result` kind `exported`.
- Step 4: refusal, `last_stage` `null`, `result` `null`.
