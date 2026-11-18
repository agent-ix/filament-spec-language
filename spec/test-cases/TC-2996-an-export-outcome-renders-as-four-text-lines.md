---
id: TC-2996
title: "An export outcome renders as its header, target, size and path"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-2996: An export outcome renders as its header, target, size and path

## Description

Verify the text form of an `export` outcome document. Scope: FR-865-AC-7.

## Test Procedure

1. Render FR-286-AC-22's counter export document written to `m.tla` as text.
2. Render the same export with no output file named.
3. Render the counter export under `export.output_bytes` 1.

## Expected Results

- Step 1: exactly `export: success (exit 0) at S4`, `target: tla filament.output.tla2/v1`, `bytes written: <n>` (the text length) and `output: m.tla`.
- Step 2: the last two lines are `bytes written: 0` and `output: none`.
- Step 3: `export: incomplete (exit 22) at S4`, `bytes written: 0`, and a line naming `export.output_bytes`.
