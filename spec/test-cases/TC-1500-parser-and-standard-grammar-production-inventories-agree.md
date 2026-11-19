---
id: TC-1500
title: "Parser and standard grammar production inventories agree"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-600
    type: verifies
---
# TC-1500: Parser and standard grammar production inventories agree

## Description

Verify that the parser's production inventory and the standard shared grammar's
`ebnf` production inventory correspond, with every unmatched name on an
allow-list with a reason, and that drift or a stale entry fails (FR-600).
Scope: FR-600-AC-1, FR-600-AC-2 and FR-600-AC-3.

## Test Procedure

1. Read `proposals/filament-v1/shared-grammar.md` through
   `filament_specification::published_file`, collect every production name of
   its `ebnf` blocks
   and compare with `fsl_cst::Production::all()` through the correspondence
   table and allow-lists in `xtask/tests/grammar_drift.rs`.
2. Run the comparison over the standard inventory plus `invented-form`, and
   over the parser inventory without `DeliveryInterval`; run it with a table
   entry naming a production absent from its side.
3. Call `published_file` for the shared grammar and for an unpublished path.

Tag the tests `#[trace("TC-1500", "<AC id>")]`.

## Expected Results

- Step 1: every name on each side is matched or allow-listed with a reason.
- Step 2: each run fails naming the unmatched production and its side, or the
  stale entry.
- Step 3: the grammar text is returned, holding `ebnf` blocks; the unpublished
  path yields `None`.
