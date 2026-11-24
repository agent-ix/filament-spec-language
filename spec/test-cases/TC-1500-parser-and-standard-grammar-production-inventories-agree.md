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
2. Run the comparison over the combined inventory plus `invented-form`, and
   over the parser inventory without `DeliveryInterval`; run it with a
   correspondence entry naming a production absent from the shared-grammar
   inventory and another naming one absent from the FR-375 inventory.
3. Run the comparison after removing the FR-375 `history-update`
   correspondence, and after changing that correspondence to a parser
   production absent from `Production::all()`.
4. Run the comparison with the FR-375 source path replaced by an unpublished
   path.
5. Call `published_file` for the shared grammar, FR-375 and an unpublished
   path.

Tag the tests `#[trace("TC-1500", "<AC id>")]`.

## Expected Results

- Step 1: every name on each side is matched or allow-listed with a reason;
  FR-375's `refinement`, row, history and fairness productions map to the
  parser variants, including the named `history-update` production. A local
  EBNF production is not accepted as parser-only solely because it is outside
  the shared-grammar file.
- Step 2: each run fails naming the unmatched production and its source, or
  the stale source-specific entry.
- Step 3: the comparison fails naming `history-update` and its FR-375 source
  when that correspondence is absent, and names both the stale FR-375
  correspondence and absent parser production when it is changed.
- Step 4: the comparison fails naming the missing FR-375 source path and the
  `history-update` production rather than passing with an incomplete combined
  inventory.
- Step 5: both grammar texts are returned holding `ebnf` blocks; the
  unpublished path yields `None`.
