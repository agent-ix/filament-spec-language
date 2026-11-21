---
id: SR-202611210022442313144108529
title: "Independent QSL-2158 integrity review"
type: SpecReview
analysis: integrity
scope: "FR-120, FR-191, FR-192, FR-203, FR-255, FR-741 changed contract at 3354b395d7739b3376cad577ddec0f4f667f1814"
review_set: subset
---

## Summary

PASS: the six changed contracts consistently distinguish the local candidate ceiling from the cumulative work allowance. No contradictory default, reset, double-charge or settlement obligation was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (placeholder) No integrity defect found in changed obligations. | FR-120-AC-35, FR-120-AC-36, FR-191-AC-8, FR-192-AC-8, FR-203-AC-13, FR-203-AC-14, FR-255-AC-23, FR-741-AC-6 |

## Analysis

FR-120 owns unit charges and run lifetime; FR-191 supplies logical order for statistical execution; FR-192 and FR-203 map existing typed stops; FR-255 owns the setting registry and defaults; FR-741 binds the native adapter. Additional candidate work is explicitly beyond existing clause work, not a replacement or duplicate clause charge. The FR-120 exception to fresh per-clause work budgets is explicit both in Inputs and clause evaluation behavior. The evaluator can receive a caller-owned meter while the caller retains cumulative accounting.

Each FR keeps its implements story edge (US-003, US-022, US-023 or US-027 as authored) and each new criterion specifies Test verification. No new stakeholder requirement, NFR, external tool invocation, pagination, external API, authentication, or undeclared package prerequisite is introduced. Full baseline traceability is not reaudited. Related obligations in a single criterion have concrete independently falsifiable outcomes; their grouping does not hide a different interpretation of the contract.

## Targeted native serial clarification follow-up

PASS at eeeb14a81f0f928d9ced4e87be9e46f21f7165ef for only the 17 inserted/two removed lines in FR-191 and FR-741, following the original frozen contract and artifact-only custody commit. Prior Findings and verdict are preserved; no new finding or disposition is required.

The native model probability route is explicitly serial, retains/reports the requested workers value, does not forward it to the model sampler and does not coerce the input to 1. Equality across requested worker counts is explicitly not evidence of multiworker execution. FR-191 scopes the worker-execution contract to routes that execute threads while retaining cumulative logical admission order for serial execution. Its generic admitted-sampler requirement continues on worker-executing routes. Existing AC identities, physical work bound, logical ordering, cancellation, local-first tie and both finite defaults are unchanged. No worker pool or sampler thread-safety implementation is claimed.

The directly affected consistency checks pass: FR-741's result equality remains compatible with serial requests; FR-191's worker-count fixtures do not assert that the native route executes workers. No settings rewrite or new wire/report member is required. Scoped quire validation passes for both FRs (2/2 grammar-clean, zero findings); diff checking passes. These results do not establish runtime compatibility or implementation correctness. Measured tools remain /home/peter/.npm-global/bin/quoin 0.36.8 and /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2).
