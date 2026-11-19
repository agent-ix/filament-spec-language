---
id: TC-3044
title: "A refused model lists every violation"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3044: A refused model lists every violation

## Description

Verify that every intake refusal and assembly error the library keeps reaches
the outcome document as its own `diagnostics` entry and the human message as
its own line. Scope: FR-056-AC-73, FR-286-AC-24.

## Test Procedure

1. Compile a unit whose `model` line selects a domain package with three
   operations, each declaring its second parameter with the first one's name
   and a different identity, so the Semantic IR reader refuses the bytes with three `DUPLICATE_PARAM` diagnostics. Build
   the `check` outcome document and serialize it twice; render the refusal's
   `Display`.
2. Repeat with the same package corrected in two of the three operations.
3. Compile a package whose two operations each repeat one parameter under
   one name and one identity.
4. Compile a unit whose assembly refuses with two errors and build its
   outcome document; render the refusal's `Display`.

## Expected Results

- Step 1: category `refusal`; `diagnostics` holds exactly three entries in
  the reader's order, each `invalid_model_binding`/`malformed-declaration`
  with `details` `null`, the locus of the unit's `model` declaration and a
  message naming `DUPLICATE_PARAM` and its own package pointer; the human
  message holds the three messages on three lines in that order and no
  ", and N more" suffix; both serializations are byte-equal.
- Step 2: one `diagnostics` entry and a one-line message.
- Step 3: two `conflicting-binding` entries, each message with a second line
  for its `DUPLICATE_PARAM` detail, and no further entry.
- Step 4: two `diagnostics` entries and a two-line message.

Test: `fsl-semantics/tests/it/model_intake.rs::a_refused_model_lists_every_violation`.
