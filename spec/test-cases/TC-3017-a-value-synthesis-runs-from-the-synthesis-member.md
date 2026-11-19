---
id: TC-3017
title: "A value synthesis runs from the synthesis member"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1037
    type: verifies
---
# TC-3017: A value synthesis runs from the synthesis member

## Description

Verify that the run request's `synthesis` member selects and runs a value
synthesis, writes the `filament.synthesis-outcome/v1` document, and refuses
bad selections and members at their `/synthesis` pointers. Scope:
FR-1037-AC-1 and FR-1037-AC-2.

## Test Procedure

1. Run `Pick` with `Pick * Pick = 49`, `= 50`, `minimize Pick` with
   `Pick >= 3`, and `= 50` with `max_candidates` 3.
2. Send `{}` over a unit holding only `Pick`, then over one holding `Pick`
   and `Other`; name `Nope` and the predicate `Positive`; send `Pick` with
   `initial`, and with `limits` `{"work": 1}`.

## Expected Results

- Step 1: `candidate` `{"type": "integer", "value": "7"}` `validated`,
  `visited` 8, exit 0; `no-candidate`, `visited` 10, exit 10; candidate 3
  `optimal`; `incomplete` naming `synthesis.max_candidates`, exit 22.
- Step 2: `Pick` runs; `missing-member` at `/synthesis/declaration`;
  `missing-name` at `/synthesis/declaration` twice; `unknown-member` at
  `/synthesis/initial` and `/synthesis/limits/work`; each exit 20, no outcome.
