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

Verify `check_hint_catalog` against the aggregate of every crate's
`CATALOG_PAIRS`, and the catalog's minimum rows. Scope: FR-286-AC-29,
FR-286-AC-30.

## Test Procedure

1. Run `check_hint_catalog(HINTS, fsl_command::diagnostic_pairs())`, and
   check that `diagnostic_pairs()` holds every pair of FR-286's "Fix hints"
   table.
2. Run it over `HINTS` plus a row `(Code::IllTyped, Some("type-mistmatch"))`.
3. Run it over `HINTS` plus a second `(Code::IllTyped, Some("type-mismatch"))` row.
4. Run it over `HINTS` plus, in turn, a row whose text holds a line feed,
   one holding U+0085, one not ending in `.`, and one of 101 `é` and a
   final `.`.
5. Call `hint_for` for every pair of the table, for
   `(Code::OutputFailure, Some("output-failure"))` and for
   `(Code::InvalidRequest, None)`.

## Expected Results

- Step 1: `Ok`; every table pair is in the aggregate.
- Step 2: `Err(HintCatalogError::UnknownPair)` naming `ill_typed` and
  `type-mistmatch`.
- Step 3: `Err(HintCatalogError::DuplicatePair)`.
- Step 4: `Err(HintCatalogError::MalformedText)` naming each row's pair; the
  last row is 203 bytes though only 102 code points.
- Step 5: a text for each of the 23 table pairs; `None` for the other two.

Test: `fsl-command/tests/it/hint_catalog.rs::the_hint_catalog_refuses_unknown_pairs_and_covers_common_mistakes`.
