---
id: TC-3197
title: "The text form names the stage in words and the JSON keeps the label"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-3197: The text form names the stage in words and the JSON keeps the label

## Description

Verify the stage word of the text header, an unknown stage, and the unchanged JSON label. Scope: FR-865-AC-9, FR-865-AC-10 and FR-865-AC-11.

## Test Procedure

1. Render as text one outcome document per `last_stage` of `S0`, `S1`, `S2`, `I1`, `I2`, `S3`, `S4`, `S5`, `S6a`, `S6b`, `S6c`, `S7` and `S8`, and one with `last_stage` null.
2. Render a document whose `last_stage` is `X9`.
3. Serialize the step 1 documents to JSON.

## Expected Results

- Step 1: the headers end ` at stage source`, ` at stage syntax`, ` at stage forms`, ` at stage intake`, ` at stage imports`, ` at stage checking`, ` at stage packaging`, ` at stage contract`, ` at stage execution`, ` at stage proof`, ` at stage model-checking`, ` at stage witness` and ` at stage replay`; the null document's header has no ` at stage `.
- Step 2: the header ends ` at stage X9`; the render does not fail; no header holds a label of the table as a word.
- Step 3: each `last_stage` is its label string.

Test: `fsl-inspect/src/render.rs` (tests module).
