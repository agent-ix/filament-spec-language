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

The compile request's `call_site` member runs FR-121's `call_site` entry and writes its refusal as the outcome's diagnostic with `selection` and `hint`.

Scope: FR-027-AC-15, FR-027-AC-16, FR-027-AC-17.

## Test Procedure

Through `fsl_command::command::compile_outcome` and the `fsl compile` driver, over FR-121-AC-32's unit and its libraries:

1. `call_site` `{"kind": "imported-function", "library": "test/other", "function": "big"}` with `test/other` supplied.
2. The same with `library` `test/absent`.
3. A unit importing only `test/wrap` (whose source imports `test/units`) with `library` `test/units`.
4. `library` `test/units` over the unit that imports it directly.
5. `kind` `function`, and an extra member.

Tag the tests `#[trace("TC-3210", "FR-027-AC-15")]`, `#[trace("TC-3210", "FR-027-AC-16")]` and `#[trace("TC-3210", "FR-027-AC-17")]`.

## Expected Results

- Steps 1 to 3: the outcome's `diagnostics` holds exactly one entry with `cause` `missing-selection`, `code` `missing_import`, `details` `null`, `hint` `Call a function of a library that the unit imports directly, or add the import.`, `locus` `null`, `selection` `unimported-library` and a `message` beginning `missing_import/missing-selection: package ` and ending ` imports no library <L>` with `<L>` `test/other`, `test/absent` and `test/units`; the text form holds the line `  hint: Call a function of a library that the unit imports directly, or add the import.`.
- Step 4: the package bytes equal those of the request without `call_site`.
- Step 5: `invalid-request` naming `call_site`.
