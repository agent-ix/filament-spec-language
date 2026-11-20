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
for an emit-only run. Scope: FR-1038-AC-3 and FR-1038-AC-4.

## Test Procedure

1. Send `seed` `-1`, `7.5` and `18446744073709551616`; send
   `"verification_plan": null`.
2. Send `{"plan": "Release", "seed": null, "emit": null, "initial": null, "max_value_bits": null, "max_trace_length": 8}`.
3. Write the outcome of a plan with a `measured` step.
4. Send `{"plan": "Release", "emit": true, "execute": false}`.
5. Send `{"plan": "Release"}`; `max_trace_length` 0, `-1`, `8.5` and `"8"`; `max_value_bits` 8 on `Release`.
6. Over the plan `Sized` (one `model_based_test` step `s` over a model with a field `n: Integer`), send `max_trace_length` 8 without `max_value_bits`, with `max_value_bits` 65, and with `max_value_bits` 8.

## Expected Results

- Step 1: `invalid-value` three times with `details.path`
  `/request/verification_plan/seed`; `wrong-value-kind` with `details.path`
  `/request/verification_plan`; each exit 20, no outcome.
- Step 2: `Release` runs; `inputs` `{"initial": null, "seed": 0, "max_trace_length": 8, "max_value_bits": null}`.
- Step 3: the disposition's members are exactly `status`, `method`,
  `evidence`, `blocked_by` and `result`; the decision is
  `result.verdict.decision.type`.
- Step 4: `status` `null`, exit 0.
- Step 5: `missing-member` with `details.path` `/request/verification_plan/max_trace_length` naming `tested`; `invalid-value` three times and `wrong-value-kind` there; `unknown-member` at `/request/verification_plan/max_value_bits`; each exit 20, no outcome.
- Step 6: `missing-member` naming `s`, then `invalid-value`, at `/request/verification_plan/max_value_bits`, exit 20; then `s` runs, its record's `provenance.max_value_bits` 8 and `inputs.max_value_bits` 8.
