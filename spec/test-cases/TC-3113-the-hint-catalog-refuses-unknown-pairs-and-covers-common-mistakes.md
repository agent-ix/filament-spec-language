---
id: TC-3113
title: "The hint catalog refuses unknown pairs and covers the common mistakes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3113: The hint catalog refuses unknown pairs and covers the common mistakes

## Description

Verify `check_hint_catalog` and the catalog's minimum rows. Scope:
FR-286-AC-29, FR-286-AC-30.

## Test Procedure

1. Run `check_hint_catalog(HINTS, catalog_pairs())`.
2. Run it over `HINTS` plus a row `(Code::IllTyped, Some("type-mistmatch"))`.
3. Run it over `HINTS` plus a second `(Code::IllTyped, Some("type-mismatch"))` row.
4. Run it over `HINTS` plus a row whose text holds a line feed, and plus a
   row whose text does not end in `.`.
5. Call `hint_for` for every pair of FR-286's "Fix hints" table, and for
   `(Code::OutputFailure, Some("output-failure"))`.

## Expected Results

- Step 1: `Ok`.
- Step 2: `Err(HintCatalogError::UnknownPair)` naming `ill_typed` and
  `type-mistmatch`.
- Step 3: `Err(HintCatalogError::DuplicatePair)`.
- Step 4: `Err(HintCatalogError::MalformedText)` naming each row's pair.
- Step 5: a text for every table pair, `invalid-request` and `io-error`
  with no cause included; `None` for the output failure.

Test: `fsl-foundation/src/diagnostic/hint.rs` (tests module).
