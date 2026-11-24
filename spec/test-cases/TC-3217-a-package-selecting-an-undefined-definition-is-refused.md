---
id: TC-3217
title: "A package selecting a definition its catalog does not define is refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: verifies
---
# TC-3217: A package selecting a definition its catalog does not define is refused

## Description

Verify that a consumer holding the unit's `DefinitionCatalog` applies the
catalog checks for a checked package, while the catalog-free strict reader
admits a structurally consistent package whose law and selection pair have
been mutated together and preserves that pair's exact opaque strings. Scope:
FR-110-AC-16.

## Test Procedure

1. Emit the package of a unit that divides with `div` under the floor law selection. Preserve the original bytes as control. In a copy, change the `definition_selections` law row's identity from `filament.value.integer-division.floor/v1` to `filament.value.integer-division.ceiling/v1` and change the corresponding `operation.laws[].definition` pair to that exact same pair. Also construct the isolated lock-row-only mutation. Run both copies through a replay request, a run request over the package and an analyze request over the package, each with the unit's catalog.
2. Change the law row's authority; in FR-110-AC-14's package replace the extension row's identity with
   one no entry holds; write a layer identity in a law row's position; change
   two rows.
3. Read each altered package with the catalog-free strict reader and inspect the reconstructed `definition_selections` row and operation-law pair. Also apply the catalog-free strict reader to the unchanged control.

## Expected Results

- Step 1: all three catalog-bound requests admit the unchanged package. The lock-row-only mutation is refused `unknown_profile`/`unsupported-selection` at `/lock/definition_selections/{i}`. The paired mutation is refused `invalid_package`/`operation-law-mismatch` at the operation law's `definition`, because ceiling is not catalogued for that role.
- Step 2: the same refusal at each row's index, `wrong-selection-role` for the
  role mismatch, the first row for two defects.
- Step 3: the catalog-free strict reader admits the unchanged package and the paired mutation, and reconstructs the altered law selection and operation-law definition with exactly the serialized `{authority, identity}` strings, including the uncatalogued ceiling identity. The isolated lock-row-only mutation is refused as structurally inconsistent.
