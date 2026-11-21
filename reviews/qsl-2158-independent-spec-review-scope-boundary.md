---
id: SR-202611210022442313144108534
title: "Independent QSL-2158 scope and boundary review"
type: SpecReview
analysis: scope-boundary
scope: "Six changed cumulative work and native default contracts at 3354b395d7739b3376cad577ddec0f4f667f1814"
review_set: subset
---

## Summary

PASS: accounting ownership, worker coordination, exact admission and native reporting responsibilities are clear. No additional wire field, quota, engine admission or result shape is required by this change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (placeholder) No changed boundary defect found. | FR-120, FR-191, FR-192, FR-203, FR-255, FR-741 |

## Responsibility allocation

| Requirement | Owner | Class |
| --- | --- | --- |
| FR-120 | Model simulator and its caller-owned run meter | Core |
| FR-191 | Statistical engine admission coordinator | Core |
| FR-192 | Statistical settlement adapter | Cross-cutting |
| FR-203 | Exact product builder and exact settlement | Core |
| FR-255 | Settings registry | Cross-cutting |
| FR-741 | Native analyze probability adapter | Core |

Caller configuration flows through native analyze to the two engines and simulator; typed stops return through settlement to the native terminal result. External shared accounting/terminal contracts (FSpec FR-461/FR-331 and shared value accounting) retain their existing meaning and are guaranteed through named setting/cause parity tests, not new external service assumptions. No new external actor, callback, storage or network dependency is introduced. Native item boundaries define independent allowances; expansions, clauses, samples and tests inside an item cannot renew them. Local ceiling provenance remains independent.
