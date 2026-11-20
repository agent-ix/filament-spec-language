---
id: TC-3116
title: "The outcome schema declares every member its writers emit"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3116: The outcome schema declares every member its writers emit

## Description

Verify the schema completeness test (every member path a writer can emit
is declared) and the corpus coverage test (every declared member path
appears in a golden document). Scope: FR-286-AC-31.

## Test Procedure

1. Run the completeness test over every writer of FR-286's "Published
   schema" table, with full-member values.
2. Validate the golden `execute` document of FR-963-AC-1's call with
   `requirements`, an FR-990-AC-1 `analyze` item, an FR-990-AC-3 item with
   `recording`, FR-634-AC-1's simulation item, a forms-refusal diagnostic
   with `construct`, `declaration`, `profile` and `text`, and an FR-650
   record value whose `fields` hold the names `a` and `zz`.
3. Run the completeness test against a copy of the schema with
   `max_messages` removed from the item definition.
4. Run the corpus coverage test against a copy of the schema declaring a
   diagnostic member `unused`.
5. Add an optional field to `OutcomeItem` and leave the full-member
   fixture unchanged; then set it in the fixture without adding it to
   `MEMBERS`; then add it to `MEMBERS` only. Make a Value-built writer emit
   a key outside its `MEMBERS`.

## Expected Results

- Step 1: no undeclared path.
- Step 2: every document validates.
- Step 3: the test fails naming `/items/*/max_messages`.
- Step 4: the test fails naming `/diagnostics/*/unused`.
- Step 5: a compile error; then the `MEMBERS` test fails; then the
  completeness test fails naming the new path; the Value-built writer's
  `MEMBERS` test fails.

Test: `fsl-command/tests/it/outcome_schema.rs::the_outcome_schema_declares_every_member_its_writers_emit`.
