---
id: TC-3210
title: "A compile request's call-site refusal reaches the user with its hint"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-121
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3210: A compile request's call-site refusal reaches the user with its hint

## Description

The compile request's `call_site` member locates its selection over the command's own checked package and writes a refusal as the outcome's diagnostic with `selection` and `hint`.

Scope: FR-027-AC-15 to FR-027-AC-21.

## Test Procedure

Through `fsl_command::command::compile_outcome` (`fsl-command/tests/it/cli_contract/compile.rs`) and the text rendering of its document (`fsl-command/tests/it/selection_text.rs`), over FR-121-AC-32's unit and libraries:

1. `call_site` `{test/other, big}` with `test/other` supplied.
2. The same with `test/absent`, and `test/units` for a unit importing only `test/wrap`.
3. `{test/units, nope}` and `{test/units, first}` (FR-121-AC-36's fixtures).
4. `{test/units, big}` over the unit that imports it directly.
5. `null`, a non-object, a missing member, a non-string value, an extra member, `kind` `function`, a model-unit program and an FR-480 array.

The driver surface of `fsl compile` is tested in the filament-driver repository (FR-865-AC-16). Tag the tests `#[trace("TC-3210", "FR-027-AC-15")]` and likewise for AC-16 to AC-21. The test `the_unimported_library_producer_text_holds_its_hint` in `selection_text.rs` builds `UnknownImport` by hand and is tagged FR-865-AC-19; the code change re-points it through `compile_outcome` (step 1) and re-tags it, and until then it does not verify FR-865-AC-19.

## Expected Results

- Step 1: AC-15's diagnostic, byte for byte, and no file at `output` and no `handoff` directory; category `refusal`, exit 20, stage I1; the text form holds `  hint: Call a function of a library that the unit imports directly, or add the import.`.
- Step 2: AC-15's diagnostic naming `test/absent` and `test/units`.
- Step 3: AC-18's and AC-19's diagnostics (`missing_declaration`/`missing-name`, `ill_typed`/`operator-ineligible`) with their hints and messages, no `selection`.
- Step 4: the package bytes of the request without `call_site`.
- Step 5: `invalid-request` naming `call_site`.
