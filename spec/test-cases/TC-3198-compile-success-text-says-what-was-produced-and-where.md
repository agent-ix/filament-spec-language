---
id: TC-3198
title: "The text of a compile success says what was produced and where it was written"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-3198: The text of a compile success says what was produced and where it was written

## Description

Verify the text form of a `packaged` compile result from the document alone. Scope: FR-865-AC-12 to FR-865-AC-16.

## Test Procedure

1. Render FR-286-AC-39's document of the request with `output` (request file in directory `reqs`).
2. Render the document of the same request without `output` and `handoff`.
3. Render the document of the request with `handoff` and no `output`.
4. Render the document of a model-unit compile that succeeds at `I1`, and a `package` document with `result` `null`.
5. Run `fsl compile <request>` and `fsl compile <request> --output <file>` in text mode and with `--format json`.

## Expected Results

- Step 1: exactly `package: success (exit 0) at stage packaging`, `produced: checked package (<n> bytes)` and `written to: reqs/spine.package.json`.
- Step 2: the same first two lines and `nothing written: no --output option, output member or handoff member was given`.
- Step 3: the first two lines and `handoff written to: reqs/<directory>`, no `written to:` line.
- Step 4: the header ends ` at stage intake`; the null-result document is the header line only.
- Step 5: the text equals `render_outcome` over the JSON document, and exit is 0.

Test: `fsl-inspect/src/render.rs` (tests module); the driver half is a Driver test in the filament-driver repository.
