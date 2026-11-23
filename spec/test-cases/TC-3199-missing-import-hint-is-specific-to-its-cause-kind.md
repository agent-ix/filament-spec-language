---
id: TC-3199
title: "The missing_import hint in text is specific to its cause kind"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-3199: The missing_import hint in text is specific to its cause kind

## Description

Verify the text hint line of each missing_import cause kind and that the JSON hint is unchanged. Scope: FR-865-AC-13 to FR-865-AC-20.

## Test Procedure

1. Render as text the refusals of FR-866-AC-3 (model), FR-866-AC-2 without its library (library), a sibling-unit import, FR-862's missing byte-provision entry, a function request naming an unimported `function_owner`, and FR-1044's non-dependency component package.
2. Render as text a `missing_import`/`missing-selection` diagnostic read from JSON.
3. Serialize each step 1 refusal to JSON.

## Expected Results

- Step 1: each text holds its kind's hint line from FR-865's table (model, library, sibling-unit, byte-provision, function-owner, component-package) and no line `  hint: Import the package`.
- Step 2: `  hint: Import the package that supplies the selected name.`.
- Step 3: `hint` is `Import the package that supplies the selected name.` and the bytes equal those written before the change.

Test: `fsl-replay`, `fsl-analyze` and `fsl-command` tests (one per producer).
