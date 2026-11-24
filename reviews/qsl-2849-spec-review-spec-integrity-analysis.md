---
id: SR-3639003
title: "spec-review (integrity, EARS) of PR 3639 new acceptance criteria"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR 3639 head bc7fa1b398; FR-126, FR-203, FR-247, FR-264 and TC-521, TC-638, TC-702, TC-738"
review_set: subset
---

## Summary

Ticket: QSL-2849. Spec review of the five new ACs, the edited FR-247 statement and use
case, and the four TC edits. `quire validate` on the nine changed spec files returned no
findings. Each AC is atomic, measurable (counted rather than timed) and matches the code.
FR-247's new SHALL ("count every recurrence step") agrees with FR-255's
`schedule.max_demand_points` row and FR-248's linear-check statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-738's new procedure step 6 sits after the "Tag the tests" paragraph, outside the numbered procedure list, so it renders as a separate list after the tagging instructions | spec/test-cases/TC-738-a-deep-package-emits-reads-back-and-verifies.md:41-44 |

## Verdict

PASS with one low layout nit.

## Dispositions

Round 1, reviewed head 1946b4382f.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Step 6 now sits inside the numbered procedure, before the Tag paragraph |

Round 2 reviewed `1ac4da2a725290e8986f265eeb45bf73917e7d0d`. The PR requires another fix round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | FR-126-AC-26 explicitly expects two full initial-state constructions at max_states=1. TC-521 and protocol_system.rs:726 verify that behavior. The owner requires the second distinct state to remain unconstructed. Align the AC and its counter oracle, preserving duplicate coalescing and the typed failure ordering. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | FR-126-AC-26 explicitly expects two full initial-state constructions at max_states=1. TC-521 and protocol_system.rs:726 verify that behavior. The owner requires the second distinct state to remain unconstructed. Align the AC and its counter oracle, preserving duplicate coalescing and the typed failure ordering. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:608 |
