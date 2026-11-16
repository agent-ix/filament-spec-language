---
id: TC-2509
title: "Duplicate protocol names are refused, and every protocol refusal lies inside its declaration"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-884
    type: verifies
---
# TC-2509: Duplicate protocol names are refused, and every protocol refusal lies inside its declaration

## Description

Verify duplicate protocol names in one unit and across units, and the accept/reject locus corpus. Scope: FR-884-AC-4, FR-884-AC-5.

## Test Procedure

1. Check a unit with two protocols `Flow`, a unit with `Flow` and `Drain`, and two units that each declare `Flow`.
2. Check the corpus of one minimal accepted and one minimal rejected protocol per S3 refusal cause.

## Expected Results

- Step 1: two `ambiguous-name` refusals at each `Flow`, each naming the other; the other units check.
- Step 2: every accepted protocol checks; every rejected protocol's region lies inside and is shorter than its declaration, except the no-profile case.
