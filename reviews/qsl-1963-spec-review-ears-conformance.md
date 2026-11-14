---
id: SR-202611141459002
title: "EARS review of the QSL-1963 FR-495 Kernel-mode statement"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; PR #2707 at 3b0bc910d964198a45d79d8f7c6f4489b93fc179, diff origin/main...HEAD: spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md (bullet \"What CheckMode::Kernel leaves unrecorded, and what it keeps\", lines 156-180)"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
---

## Summary

Ticket: QSL-1963. This review checks the one new requirement statement in the diff, the
four-boundary Kernel bullet in FR-495, against EARS grammar. FR-495's own requirement bullets
use EARS with SHALL: "S3 SHALL refuse each declaration whose form the layer does not admit"
and "the checker SHALL record no range or nonzero obligation".

## Verdict

PASS with one low finding. Each boundary names one condition (a mode, a construct) and one
response, so the statements are atomic and state-driven ("Under CheckMode::Kernel ..."). They
are written as indicative description, not as SHALL requirements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new Kernel bullet states its four boundaries in the indicative ("refuses ... and checks under `Kernel`", "refuses its unproved obligation as it does under `Linked`", "are checked as under `Linked` in both modes", "refuse under `Kernel` exactly as under `Linked`") with no SHALL. Neighbouring FR-495 requirement bullets use EARS with SHALL, so these read as commentary, not obligations. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:156 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
