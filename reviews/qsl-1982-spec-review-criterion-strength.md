---
id: SR-202611141554003
title: "Criterion strength review of the QSL-1982 FR-495 criteria"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/quire-spec-language; PR #2708 at 402ca8da65aef4aaafa016888cd1ac451d8e7a77, diff origin/main...HEAD: FR-495-AC-14, FR-495-AC-15, FR-495-AC-16"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
---

## Summary

Ticket: QSL-1982. Jev (typesafe.ai System One) is not installed on this host (`jev` is not on
PATH), so I judged this lens by hand over the spec text and the PR #2706 code recorded in
SR-202611141554001.

## Verdict

PASS. Each criterion names the Linked refusal (code, cause, location, "as its only refusal")
and a clean Kernel check, so it fails if Kernel records the obligation or Linked drops it:
- AC-14 fails if Kernel keeps `RationalRange`.
- AC-15 fails if Kernel keeps the rational `/` nonzero obligation. It also fails if Linked
  checks the result domain before the divisor, because then the refusal would be
  `unproved-range`, not `unproved-nonzero`.
- AC-16 fails if Kernel keeps `NonemptyReduction`. Its `[1, 2]` control is a positive case:
  it fails if the checker stopped using the collection's lower size bound and refused every
  identity-free `reduce`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
