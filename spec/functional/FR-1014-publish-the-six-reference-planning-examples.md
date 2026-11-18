---
id: FR-1014
title: "Publish the six reference planning cases as runnable examples"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-810
    type: depends_on
---
# FR-1014: Publish the six reference planning cases as runnable examples

## Description

FSL SHALL run FSpec FR-810's six reference cases from the fixtures the standard publishes (FSpec FR-810, `proposals/planning-v1/fixtures/<case>/`: `unit.native`, `model.semantic-ir.json`, `snapshot.json`, `plan.json`), and SHALL publish under `examples/planning/<case>/` only its own host wrapping of each, a `native-run/1` `request.json` whose `program`, `models` and `synthesis` members name those four files as siblings. The fixtures are the standard's conformance content, so FSL holds no copy of the unit, the model or the snapshot, and its envelope's `synthesis` member equals `plan.json` member for member; a test SHALL materialize each case's published files beside its `request.json`, run it, and compare the outcome with FSpec FR-810's stated plan and objective.

## Behavior

- Each example directory SHALL be named for its case (`o1-rollout`, `o2-battery-dispatch`, `o3-production-schedule`, `o4-staffing`, `o5-robot-routing`, `o6-autoscaling`).
- The test SHALL evaluate every returned state against the case's rules with the native evaluator.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1014-AC-1 | Each of the six examples plans with the result and exact objective of FSpec FR-810-AC-1 to AC-6 (lengths 15 and 10 for O1 and O5, `0.30` for O2, `60 min` for O3, 20 for O4, `0.80` for O6), and every returned state satisfies the case's rules. | Test (TC-2864) |
| FR-1014-AC-2 | Each `examples/planning/<case>/` directory holds `request.json` and no copy of `unit.native`, `model.semantic-ir.json`, `snapshot.json` or `plan.json`; its `synthesis` member equals the published `plan.json`, member for member, and `filament_specification::published_file` returns each of the four files for that case. | Test (TC-2864) |

## Dependencies

- FR-810 (filament-specification).
