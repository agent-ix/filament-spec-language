---
id: TC-3018
title: "A verification plan runs from the verification plan member"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1038
    type: verifies
---
# TC-3018: A verification plan runs from the verification plan member

## Description

Verify that the run request's `verification_plan` member selects one plan,
supplies `initial` and `seed`, and writes the plan run outcome, and that bad
selections and members refuse at their pointers. Scope: FR-1038-AC-1 and
FR-1038-AC-2.

## Test Procedure

1. Execute `Release` with `initial` and `seed` 7; then emit it without
   executing.
2. Send `{}` with one plan and with two; name `Nope` and `Never`; send
   `seed` `"7"`.

## Expected Results

- Step 1: `safe` refuted, `tested` tested, `timely` blocked by `safe`,
  `failed_steps` `["safe"]`, exit 22; then the plan document, exit 0.
- Step 2: `Release` runs; `missing-member`, then `missing-name` twice, at
  `/request/verification_plan/plan`; `wrong-value-kind` at
  `/request/verification_plan/seed`; each exit 20.
