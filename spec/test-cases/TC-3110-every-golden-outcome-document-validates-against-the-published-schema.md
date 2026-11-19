---
id: TC-3110
title: "Every golden outcome document validates against the published schema"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-3110: Every golden outcome document validates against the published schema

## Description

Verify that the outcome schema `urn:filament:outcome:1` accepts every golden
outcome document and rejects mutated documents at the named instance
pointer. Scope: FR-286-AC-25.

## Test Procedure

1. Rebuild the document of each file of
   `fsl-replay/tests/fixtures/outcome-golden/` and compare its `to_bytes`
   output with the file; validate each file against `OUTCOME_SCHEMA_1`.
2. Check that the corpus holds a document of every `Operation`, every
   `result` kind and every `Counterexample` variant.
3. Mutate FR-286-AC-2's golden document four ways: remove
   `/diagnostics/0/code`; add a member `fix` to `/diagnostics/0`; set
   `/category` to `"violated"`; set `/diagnostics/0/hint` to `""`.
   Validate each.
4. Remove `aggregate` from FR-520-AC-1's `analyze` document; add an
   `aggregate` member to FR-286-AC-1's `check` document. Validate each.

## Expected Results

- Step 1: every file equals its rebuilt bytes and validates.
- Step 2: no operation, result kind or variant is missing.
- Step 3: each mutation fails, with errors at `/diagnostics/0` (keyword
  `required`), `/diagnostics/0` (unevaluated member `fix`), `/category` and
  `/diagnostics/0/hint` respectively.
- Step 4: both fail with an error at the root pointer.

Test: `fsl-replay/tests/it/outcome_schema.rs::every_golden_outcome_validates_and_mutations_fail_at_their_pointer`.
