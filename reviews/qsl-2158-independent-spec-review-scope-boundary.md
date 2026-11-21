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

## Targeted native serial clarification follow-up

PASS at eeeb14a81f0f928d9ced4e87be9e46f21f7165ef for only the 17 inserted/two removed lines in FR-191 and FR-741, following the original frozen contract and artifact-only custody commit. Prior Findings and verdict are preserved; no new finding or disposition is required.

The native model probability route is explicitly serial, retains/reports the requested workers value, does not forward it to the model sampler and does not coerce the input to 1. Equality across requested worker counts is explicitly not evidence of multiworker execution. FR-191 scopes the worker-execution contract to routes that execute threads while retaining cumulative logical admission order for serial execution. Its generic admitted-sampler requirement continues on worker-executing routes. Existing AC identities, physical work bound, logical ordering, cancellation, local-first tie and both finite defaults are unchanged. No worker pool or sampler thread-safety implementation is claimed.

The directly affected consistency checks pass: FR-741's result equality remains compatible with serial requests; FR-191's worker-count fixtures do not assert that the native route executes workers. No settings rewrite or new wire/report member is required. Scoped quire validation passes for both FRs (2/2 grammar-clean, zero findings); diff checking passes. These results do not establish runtime compatibility or implementation correctness. Measured tools remain /home/peter/.npm-global/bin/quoin 0.36.8 and /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2).
