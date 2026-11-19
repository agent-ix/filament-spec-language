---
id: FR-1038
title: "Read the run request's verification plan member and run the selected plan"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-753
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1037
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-824
    type: depends_on
---
# FR-1038: Read the run request's verification plan member and run the selected plan

## Description

FSL SHALL read a `native-run/1` request's `verification_plan` member as
FSpec FR-824's verification plan request, select the plan it names from the
plan documents FR-753 emits for the run's package, hand `initial` and
`seed` to the steps FSpec FR-824 says consume them, run the plan by FR-753's
coordinator when `execute` holds, and write FSpec FR-824's
`filament.verification-plan-outcome/v1` document as the run outcome's
`verification_plan` member, with the exit FSpec FR-824 assigns.

## Behavior

- The `verification_plan` member SHALL be the run request's one plan
  member, read by `fsl-command`'s strict reader before any stage runs for
  its closed member list and JSON kinds. Every refusal SHALL be located at
  FSpec FR-824's pointer prefixed by `/request/verification_plan`, the
  member's pointer in the `native-run/1` document the caller sent, as
  FR-1015 locates `export` refusals under `/request/export` and FR-1039
  locates `package` refusals under `/request/package`: FSpec FR-824's
  `/seed` is reported as `/request/verification_plan/seed`, and the whole
  member's defects at `/request/verification_plan`. The diagnostic carries
  the pointer as `details.path` with the FSpec FR-805 cause as its `cause`,
  code `invalid_runtime_input`, as every other run-request member refuses.
- A member of the `verification_plan` object whose value is JSON `null` is
  read as absent, as the `export` reader reads it (FSpec FR-824); a `null`
  `verification_plan` member is not an object and refuses
  `wrong-value-kind` at `/request/verification_plan`. A `seed` that is a
  JSON number but not an integer literal from 0 to 2^64 − 1 (negative,
  fractional, exponent form, too large) refuses `invalid-value`; a `seed`
  of another JSON kind refuses `wrong-value-kind`.
- A `measured` step's decision is written only in its disposition's
  `result`, at `result.verdict.decision`, and the disposition carries no
  other member than FSpec FR-824's five; a run with `execute` `false`
  writes `status` `null`.
- FSL SHALL select the plan as FSpec FR-824 "Selection" states, by the
  emitted plan documents' `plan.qualified_name`, using the same lookup and
  refusals FR-1037 uses for a synthesis declaration, so an omitted name
  runs the unit's only plan.
- FSL SHALL resolve `initial` against the request file's directory and
  admit it for each consuming step's model with that model's snapshot
  admission, and SHALL give each `model_based_test` and
  `probabilistic_statistical` step the request's `seed` (default `0`).
- FSL SHALL refuse `initial` or `seed` on a selected plan with no step that
  consumes it, `unknown-member` at its pointer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1038-AC-1 | `{"plan": "Release", "execute": true, "initial": "spine-snapshot.json", "seed": 7}` over FR-753-AC-4's unit writes `verification_plan.format` `filament.verification-plan-outcome/v1`, `safe` `refuted`, `tested` with `evidence` `tested`, `timely` `blocked` by `safe`, `failed_steps` `["safe"]` and `inputs.seed` 7, exit `22`, FSpec FR-301's highest-severity code of the refuted and the blocked step; `{"plan": "Release", "emit": true, "execute": false}` writes the plan document with three nodes and one `requires` edge, exit `0`. | Test (TC-3018) |
| FR-1038-AC-2 | `{}` over the unit's only plan runs `Release`; with a second plan `Other` it refuses `invalid_runtime_input`/`missing-member` at `/request/verification_plan/plan`; `plan` `Nope` and `plan` `Never` (an invariant) each refuse `missing_declaration`/`missing-name` there; `seed` `"7"` refuses `wrong-value-kind` at `/request/verification_plan/seed`; each exits `20` with no `verification_plan` outcome. | Test (TC-3018) |
| FR-1038-AC-3 | Over FR-753-AC-4's unit, `seed` `-1`, `7.5` and `18446744073709551616` each refuse `invalid_runtime_input`/`invalid-value` with `details.path` exactly `/request/verification_plan/seed`, and `"verification_plan": null` refuses `wrong-value-kind` with `details.path` exactly `/request/verification_plan`, each exit `20` with no outcome; `{"plan": "Release", "seed": null, "emit": null, "initial": null}` runs `Release` with `inputs` `{"initial": null, "seed": 0}`; a `measured` disposition has exactly the members `status`, `method`, `evidence`, `blocked_by` and `result`, its decision at `result.verdict.decision.type`; `{"plan": "Release", "emit": true, "execute": false}` writes `status` `null`, exit `0`. | Test (TC-3032) |

## Dependencies

- FR-753 (the plan and coordinator); FR-1037 (the shared selection);
  FSpec FR-824.
