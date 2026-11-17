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

FSL SHALL publish FSpec FR-810's six reference cases as examples under `examples/planning/`, each a source unit, a model, a snapshot and a plan request, and a test SHALL run each and compare the outcome with FSpec FR-810's stated plan and objective.

## Behavior

- Each example directory SHALL be named for its case (`o1-rollout`, `o2-battery-dispatch`, `o3-production-schedule`, `o4-staffing`, `o5-robot-routing`, `o6-autoscaling`).
- The test SHALL evaluate every returned state against the case's rules with the native evaluator.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1014-AC-1 | Each of the six examples plans with the result and exact objective of FSpec FR-810-AC-1 to AC-6 (lengths 15 and 10 for O1 and O5, `0.30` for O2, `60 min` for O3, 20 for O4, `0.80` for O6), and every returned state satisfies the case's rules. | Test (TC-2864) |

## Dependencies

- FR-810 (filament-specification).
