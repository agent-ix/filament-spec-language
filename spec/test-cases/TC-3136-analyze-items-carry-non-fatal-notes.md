---
id: TC-3136
title: "Analyze items carry non-fatal notes"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-3136: Analyze items carry non-fatal notes

## Description

Verify that an analyze item reports the sampled witnesses its verdict does
not write in its `notes` member, and that a note changes nothing else about
the item. Scope: FR-621-AC-15.

## Test Procedure

1. Analyze a statistical `probability` item over FR-193's violating model
   with `max_witnesses` 2, then with `max_witnesses` 8.
2. Analyze the same item with a kept witness one of whose members has no
   FSpec FR-410 spelling, beside a second such witness.
3. Remove `notes` from each item and compare its record, category, exit
   code and identities with the original item's.

Tag the tests `#[trace("TC-3136", "FR-621-AC-15")]`.

## Expected Results

- Step 1: two `witnesses` and one `witnesses-beyond-limit` note with
  `kept` `"2"` and `found` `"5"`; with `max_witnesses` 8, no `notes` member.
- Step 2: each such witness is absent from `witnesses` and named by one
  `witness-omitted` note with its `test`, `trace` and `at`; the notes are
  in `code`, `test`, `trace` order.
- Step 3: every compared member is equal.
