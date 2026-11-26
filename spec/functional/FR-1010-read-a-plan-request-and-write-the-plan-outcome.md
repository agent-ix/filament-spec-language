---
id: FR-1010
title: "Read a plan request from a run request and write the plan outcome document"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-805 filament-specification/FR-650 filament-spec-language/FR-755
    type: depends_on
---
# FR-1010: Read a plan request from a run request and write the plan outcome document

## Description

FSL SHALL read a `native-run/1` request's `synthesis` member over a state-model grammar as FSpec FR-805's plan request, and SHALL write FSpec FR-805's `filament.plan-outcome/v1` document as the run outcome's `synthesis` member, with the exit code FSpec FR-805 assigns its status.

## Behavior

- The `synthesis` member SHALL carry exactly FSpec FR-805's plan request members other than `format` and `package`: `declaration`, `initial` (the snapshot file), `policy`, `limits` (FSpec FR-801's member names: `max_states`, `max_work`, `max_certificate_entries`, `max_check_work`, `max_state_key_bytes`), `accounting`, `deadline_ms` and, for a hybrid grammar only, `dwell_grid`, read with `fsl-command`'s strict reader before any stage runs. The list is closed: an engine choice and the planner's test seams are library parameters (`run_with_providers`, `run_with_plan_fault`), never members, so a request cannot forge a replay or select an engine its caller did not register.
- The `synthesis` member SHALL be FSpec FR-823's one synthesis request for every synthesis kind: the selected declaration's grammar decides whether it carries this requirement's plan members or FR-1037's value members, and `declaration` SHALL be selected by FSpec FR-823 "Selection" (omitted, it runs the package's only `synthesis` declaration). A value-type declaration named here runs as a value synthesis (FR-1037), never as a plan.
- Every refusal pointer SHALL be FSpec FR-805's plan-request pointer prefixed by `/request/synthesis`, the member's own pointer in the `native-run/1` document the caller sent: `/request/synthesis/limits/max_work`, `/request/synthesis/policy/0`, `/request/synthesis/initial`, `/request/synthesis/dwell_grid`.
- The run outcome's `synthesis` member SHALL be the plan outcome document with every member FSpec FR-805 "Members by status" lists present and `null` where the status gives it no value, `objective` the object `{direction, expression, value}`, and every value in FSpec FR-181's typed value form, as FSpec FR-805 and FR-323 require of the native runtime's outcome: an integer objective as `{"type": "integer", "value": "3"}`, a Decimal as `{"type": "decimal", "coefficient": "30", "scale": "2"}`, a Rational as `{"type": "rational", "numerator": "3", "denominator": "4"}` and a quantity such as `elapsed` as FR-181's unit-bearing quantity, the normalized `rational` form with its `unit` (`60 min` as `{"type": "rational", "numerator": "60", "denominator": "1", "unit": "min"}`).
- An `incomplete` outcome's `cause` SHALL be one of `limit-reached`, `cancelled`, `certificate-rejected` and `invariant-undetermined`; the last is written `{"type": "invariant-undetermined", "invariant": <name>}` (FR-1012).
- An `inconclusive` outcome SHALL carry cause `{"type":"no-instance"}`, null `plan`, `replay`, `certificate`, `certification`, `bound` and `objective.value`, and zero explored states, work, NaN objectives and undefined objectives.
- The exit SHALL be `0` for `optimal` and `feasible`, `10` for `infeasible`, `22` for `incomplete` and `inconclusive`, and `21` for `unsupported` (a hybrid plan with no hybrid candidate, FSpec FR-819). A refusal SHALL use the exit assigned to its code category by FR-285: `incomplete_population` 22, `unsupported_construct` and `unsupported_projection` 21, and other refusal codes 20. A method failure exits `30`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1010-AC-1 | A plan request over the spine model writes `synthesis.format` `filament.plan-outcome/v1` with `status`, `objective`, `plan`, `replay`, `certificate`, `certification`, `explored` and `limits`, and exits `0` for an `optimal` result. | Test (TC-2860) |
| FR-1010-AC-2 | An `infeasible` result writes `plan` `null` and exits `10`; a request with an unknown `synthesis` member refuses `invalid_runtime_input`/`unknown-member` at `/request/synthesis/<member>` and exits `20`. | Test (TC-2860) |
| FR-1010-AC-3 | Over the spine model, an `infeasible` outcome writes `objective` with `direction` `minimize`, `expression` `length` and `value` `null`, and `replay` and `bound` `null`, with no outcome member absent; an `optimal` one writes `objective.value` `{"type": "integer", "value": "1"}`; a `synthesis` member carrying `engines` refuses `unknown-member` at `/request/synthesis/engines`, exit `20`. | Test (TC-2860) |
| FR-1010-AC-4 | A policy over a population with no admitted instance writes `inconclusive`, cause `{"type":"no-instance"}`, null plan, replay, certificate, certification, bound and objective value, zero explored states and work, and exits `22`. | Test (TC-2860) |

## Dependencies

- FR-805 filament-specification/FR-650 filament-spec-language/FR-755 (filament-specification).
