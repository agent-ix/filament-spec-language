---
id: TC-3032
title: "The verification plan member reads null, seed domains and decisions by one rule"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1038
    type: verifies
---
# TC-3032: The verification plan member reads null, seed domains and decisions by one rule

## Description

Verify that the `verification_plan` reader locates refusals under
`/request/verification_plan`, reads `null` members as absent, refuses a
seed outside its domain `invalid-value`, and that the outcome writer carries
a measured decision only in the step's result and writes `status` `null`
for an emit-only run. Scope: FR-1038-AC-3.

## Test Procedure

1. Send `seed` `-1`, `7.5` and `18446744073709551616`; send
   `"verification_plan": null`.
2. Send `{"plan": "Release", "seed": null, "emit": null, "initial": null}`.
3. Write the outcome of a plan with a `measured` step.
4. Send `{"plan": "Release", "emit": true, "execute": false}`.

## Expected Results

- Step 1: `invalid-value` three times with `details.path`
  `/request/verification_plan/seed`; `wrong-value-kind` with `details.path`
  `/request/verification_plan`; each exit 20, no outcome.
- Step 2: `Release` runs; `inputs` `{"initial": null, "seed": 0}`.
- Step 3: the disposition's members are exactly `status`, `method`,
  `evidence`, `blocked_by` and `result`; the decision is
  `result.verdict.decision.type`.
- Step 4: `status` `null`, exit 0.
