---
id: SR-96722
title: "QSL-3401 spec integrity review of the FR-245 ZoneEdge verdict text (filament-spec-language PR 3979)"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@0b9befd5ad62ef2bc971fee598d8c7d6a299ce6c; spec/functional/FR-245-check-a-zone-certificate-in-the-qualified-core.md"
review_set: subset
date: 2026-11-25
---

## Summary

Ticket: QSL-3401. The only spec edit adds `verdict: Option<FailedStep>` to
`ZoneEdge`, held for `StepFails` only, written as FSpec FR-331's zone edge step
`{edge, verdict}`. It matches FR-245's rule table ("`ZoneEdge` of that edge,
with the failing verdict as FR-149's locus carries it"), FR-149's
`FailedStep`, and FSpec FR-331 l.237 and AC-45. Nothing is weakened; no AC
text changed.

## Verdict

PASS with one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-245-AC-7's closing sentence lists the written forms as zone node, zone edge, zone component, State and certificate member, omitting the zone edge step; no AC states the step-fails verdict is written (TC-700 asserts it under AC-8). | spec/functional/FR-245-check-a-zone-certificate-in-the-qualified-core.md:204-205 |

## Coverage

- Statements examined: FR-245 locus paragraph (l.68-79), rule table row StepFails, FR-245-AC-7, FR-245-AC-8.
