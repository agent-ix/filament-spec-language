---
id: TC-2998
title: "Request, check and compile a model unit through the command"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-710
    type: verifies
---
# TC-2998: Request, check and compile a model unit through the command

## Description

Verify the command surface of a model unit: the library compile entry and
labelled package input, a `native-model-unit/1` entry of a request's
`models`, a model unit as a `check` model selection and as the checked
source, and a compile request that writes the compiled document. Scope:
FR-710-AC-6 to FR-710-AC-9.

## Test Procedure

1. Call `compile_model_unit` over FR-710-AC-1's model unit; build `package_input_from_sources` over it with labels (`agent-ix`, `test:shop-model`), then over it beside its compiled document.
2. Compile and run FR-710-AC-7's program with a `native-model-unit/1` `models` entry and again with the compiled document as a `semantic-ir/2.0.0` entry; then with the `Ghost` model unit, the program source and a `semantic-ir/2.0.0` entry naming `shop.fsl`.
3. Call `check_outcome` with the model selection `("example/shop", shop.fsl)`, with the compiled document file, with `("example/other", shop.fsl)` over a program selecting `example/other`, and over `shop.fsl` and the `Ghost` unit as the checked source.
4. Compile a request whose `program` is the model unit, without and with `output`, and with `models` `[]`, a `libraries` entry or `clocks`.

Test: `fsl-command/tests/it/model_unit_request.rs`.

## Expected Results

- Step 1: identity `example/shop` and the AC-1 bytes; origins name the labelled source; `invalid_package`/`conflicting-definition` naming `example/shop`.
- Step 2: equal package bytes and equal run outcomes; `missing_declaration`/`missing-name` at `Ghost` at stage `I1`; `invalid-request` naming `models`, index 0 and `native-model-unit/1`, exit 20; `invalid_model_binding`/`malformed-declaration`.
- Step 3: success at S4 equal to the document route; `wrong-model-selection`; success at `I1`; the `Ghost` refusal.
- Step 4: the package bytes equal `compile_model_unit`'s bytes with `artifacts` `[]` and the output file holds them; each extra member refuses `invalid-request` naming it, exit 20, no package bytes and no file.
