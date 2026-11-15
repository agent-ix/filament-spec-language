---
id: TC-1943
title: "Model unit refusals and package input"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-710
    type: verifies
---
# TC-1943: Model unit refusals and package input

## Description

Verify that a model unit is refused at its own spans before any document is
written, and that a package input may supply a model unit in place of a
document. Scope: FR-710-AC-4 and FR-710-AC-5.

## Test Procedure

1. Compile each FR-710-AC-4 model unit and record the refusal and its span.
2. Supply the AC-1 model unit under `example/shop` for `model M =
   "example/shop";` and check; check the same unit with the compiled document
   supplied as bytes; compare the checked packages.
3. Supply it under `example/other` for `model M = "example/other";`.

## Expected Results

Step 1 gives each FR-710-AC-4 code, cause and span, and no document. Step 2
admits both routes with equal checked packages. Step 3 refuses
`invalid_model_binding`/`wrong-model-selection`.
