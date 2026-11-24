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

Verify that a consumer holding the unit's `DefinitionCatalog` refuses a
checked package whose `definition_selections` names a definition the catalog
lacks, with the typed refusal at the row, and that the catalog-free strict
reader admits the same bytes while preserving the opaque row's exact strings.
Scope: FR-110-AC-16.

## Test Procedure

1. Emit the package of a unit that divides with `div` under the floor law selection. Run it through a replay request, a
   run request over the package and an analyze request over the package, each
   with the unit's catalog: unchanged, then with the law row `filament.value.integer-division.floor/v1` changed
   to `filament.value.integer-division.ceiling/v1`.
2. Change the law row's authority; in FR-110-AC-14's package replace the extension row's identity with
   one no entry holds; write a layer identity in a law row's position; change
   two rows.
3. Read each altered package with the strict reader alone and inspect the
   reconstructed package's `definition_selections` row at each changed index.

## Expected Results

- Step 1: the unchanged package is admitted by all three; the altered one is
  refused `unknown_profile`/`unsupported-selection` at
  `/lock/definition_selections/{i}` with no result.
- Step 2: the same refusal at each row's index, `wrong-selection-role` for the
  role mismatch, the first row for two defects.
- Step 3: every altered package is admitted; each changed row retains exactly
  the serialized `{authority, identity}` strings, including an identity not
  defined by any catalog.
