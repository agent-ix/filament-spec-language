---
id: SR-202611141401003
title: "Criterion strength review of the QSL-1942 changed ACs"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/quire-spec-language; PR #2704 at 8ef45a09e17ae92270ebd7a973209a86e40f05d3, diff origin/main...HEAD: FR-560-AC-2, FR-562-AC-3, FR-562-AC-5, FR-295-AC-1"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-560, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-562, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-295, type: references }
---

## Summary

Ticket: QSL-1942. Jev (typesafe.ai System One) is not installed on this host (`jev` not on
PATH), so this lens was judged manually over the spec text and the checker run recorded in
SR-202611141401001.

## Verdict

PASS for this lens. Each changed AC names a concrete, observable result a wrong
implementation would miss:
- FR-295-AC-1: equality of outcome, cause, `Location` and loss records per case, plus a
  seeded wrong backend that must fail naming the program; the corpus is now a fixed set.
- FR-562-AC-3: exact kernel-call counts for `d` (0) and `d0` (1), plus `Undefined`,
  `division-by-zero` and the `div`'s `Location` at `b` = 0; a translator that skips the
  kernel for `d0` or reports another cause fails.
- FR-560-AC-2 and FR-562-AC-5: the assertions (no entry unsupported; one kernel call each)
  can fail, but two of their fixtures (`half`, `r`) are refused by the checker, so as written
  neither AC can pass. That defect is recorded once, as SR-202611141401001 FND-001 and
  FND-002, and is not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
