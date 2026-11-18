---
id: FR-1011
title: "Meter the plan search under the plan_search settings"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-801
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-805
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-461
    type: depends_on
---
# FR-1011: Meter the plan search under the plan_search settings

## Description

FSL SHALL give its plan search a `PlanSearchLimits` type in `fsl-analyze` whose members are FSpec FR-801's, each with its `plan_search.<member>` setting and a published default, and SHALL charge every evaluation the search performs to the run's evaluation meter.

The request member that raises a limit is the plan request's `limits` (FSpec FR-805 "Request"), whose members are FR-801's bare member names (`max_work`, `max_states`, ...). In a `filament.plan-request/v1` document it is a top-level member; a `native-run/1` request carries the plan request's members under its `synthesis` member (FSpec FR-805's host request surface), so there the same `limits` member is `synthesis.limits`, with FR-805's meaning and refusals. The plan request has no other limits member.

## Behavior

- Each member SHALL be raisable through the library setter, the plan request's `limits` member (FSpec FR-805) and `--limit plan_search.<member>=<n>` (FSpec FR-461 Behavior 3).
- A `limits` member SHALL be named by its FR-801 member name.
- If a `limits` member has an unknown name, a zero, negative or non-integer value or a value above `u64::MAX`, then FSL SHALL refuse `invalid_runtime_input` at `/limits/<member>` before any search, as FSpec FR-805 and FR-461-AC-4 state.
- A denied charge SHALL stop the search `incomplete` with `limit-reached{limit, value, setting}`.
- `plan_search.*` SHALL be distinct from `planner.max_work` (FSpec FR-807).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1011-AC-1 | A plan request whose `limits` (FSpec FR-805) is `{"max_work": 1}` stops the plan search `incomplete` naming `plan_search.max_work`, exit `22`; `{"max_states": 1}` names `plan_search.max_states`. | Test (TC-2861) |
| FR-1011-AC-2 | A low `accounting.work_units` names `accounting.work_units`, not a `plan_search` setting. | Test (TC-2861) |
| FR-1011-AC-3 | A plan request whose `limits` is `{"max_steps": 1}` refuses `invalid_runtime_input`/`unknown-member` at `/limits/max_steps`, and `{"max_work": 0}` refuses `invalid-value` at `/limits/max_work`, each exit 20 with no search; a request with no `limits` reports every `plan_search.*` limit at its published default in the outcome's `limits`. | Test (TC-2861) |

## Dependencies

- FSpec FR-801 (the limits), FR-805 (the plan request and its `limits` member), FR-461 (setting names and refusals).
