---
id: SR-202611141554002
title: "EARS review of the QSL-1982 FR-495 Kernel bullet edit"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; PR #2708 at 402ca8da65aef4aaafa016888cd1ac451d8e7a77, diff origin/main...HEAD: spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md (lead sentence of the bullet \"What CheckMode::Kernel leaves unrecorded, and what it keeps\", line 157)"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
---

## Summary

Ticket: QSL-1982. The only requirement-statement change is a parenthetical list added to the
Kernel bullet's lead sentence. It names the five obligation kinds Kernel drops. The SHALL NOT
that follows it, and the SHALL boundaries merged in QSL-1963, are unchanged. AC-14..16 are
criteria, not EARS statements.

## Verdict

PASS. The list makes the lead sentence's scope explicit without adding a second response or
condition. The bullet stays one state-driven requirement ("Under CheckMode::Kernel ...")
whose obligations SHALL be met as stated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
