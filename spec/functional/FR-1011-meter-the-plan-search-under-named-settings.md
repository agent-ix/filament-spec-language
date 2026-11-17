---
id: FR-1011
title: "Meter the plan search under the plan_search settings"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-801 filament-specification/FR-461
    type: depends_on
---
# FR-1011: Meter the plan search under the plan_search settings

## Description

FSL SHALL give its plan search a `PlanSearchLimits` type in `fsl-analyze` whose members are FSpec FR-801's, each with its `plan_search.<member>` setting and a published default, and SHALL charge every evaluation the search performs to the run's evaluation meter.

## Behavior

- Each member SHALL be raisable through the library setter, the request's `synthesis.limits` and `--limit plan_search.<member>=<n>` (FSpec FR-461 Behavior 3).
- A denied charge SHALL stop the search `incomplete` with `limit-reached{limit, value, setting}`.
- `plan_search.*` SHALL be distinct from `planner.max_work` (FSpec FR-807).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1011-AC-1 | `synthesis.limits.max_work` 1 stops the plan search `incomplete` naming `plan_search.max_work`, exit `22`; `max_states` 1 names `plan_search.max_states`. | Test (TC-2861) |
| FR-1011-AC-2 | A low `accounting.work_units` names `accounting.work_units`, not a `plan_search` setting. | Test (TC-2861) |

## Dependencies

- FR-801 filament-specification/FR-461 (filament-specification).
