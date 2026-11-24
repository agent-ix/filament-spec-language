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

Verify that the parser's production inventory corresponds to both the standard
shared grammar's and the explicitly referenced normative FR-local `ebnf`
production inventories, with every remaining unmatched name on an allow-list
with a reason, and that drift or a stale entry fails (FR-600). Scope:
FR-600-AC-1 through FR-600-AC-4.

## Test Procedure

1. Read `proposals/filament-v1/shared-grammar.md` and
   `spec/functional/analysis/FR-375-declare-model-refinement.md` through
   `filament_specification::published_file`; collect production names from the
   shared grammar's and FR-375's `ebnf` blocks, then compare the combined
   inventory with `fsl_cst::Production::all()` through the source-aware
   correspondence tables and allow-lists in `xtask/tests/grammar_drift.rs`.
2. Run the comparison over the standard inventory plus `invented-form`, and
   over the parser inventory without `DeliveryInterval`; run it with a table
   entry naming a production absent from its side.
3. Call `published_file` for the shared grammar, FR-375 and an unpublished
   path.

Tag the tests `#[trace("TC-1500", "<AC id>")]`.

## Expected Results

- Step 1: every name on each side is matched or allow-listed with a reason;
  FR-375's `refinement`, row, history and fairness productions map to the
  parser variants, including the named `history-update` production. A local
  EBNF production is not accepted as parser-only solely because it is outside
  the shared-grammar file.
- Step 2: each run fails naming the unmatched production and its side, or the
  stale entry.
- Step 3: both grammar texts are returned holding `ebnf` blocks; the
  unpublished path yields `None`.
