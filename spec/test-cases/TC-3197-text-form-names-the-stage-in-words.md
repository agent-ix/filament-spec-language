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

Verify that the text header writes a stage word and never a label, and that the JSON document keeps its labels. Scope: FR-865-AC-9 and FR-865-AC-10.

## Test Procedure

1. Render as text one outcome document per `last_stage` of `S0`, `S1`, `S2`, `I1`, `S3`, `S4`, `S5`, `S6a`, `S6b`, `S6c`, `S7` and `S8`, and one with `last_stage` null.
2. Serialize the same documents to JSON.

## Expected Results

- Step 1: the headers end ` at stage source`, ` at stage syntax`, ` at stage forms`, ` at stage intake`, ` at stage checking`, ` at stage packaging`, ` at stage contract`, ` at stage execution`, ` at stage proof`, ` at stage model-checking`, ` at stage witness` and ` at stage replay`; no text line holds a stage label as a word; the null document's header has no ` at stage `.
- Step 2: `last_stage` is the label, and the bytes equal those written before the change.

Test: `fsl-inspect/src/render.rs` (tests module).
