---
id: SR-2026111608544802
title: "Integrity review of the FSL wave 5 spec readiness batch"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #2744 branch fsl-spec-w5 at f38affa78, git diff origin/main...HEAD (FR-132, FR-255, FR-267, FR-501, FR-624, FR-625, FR-627, FR-640, FR-641, NFR-017, TC-1160, TC-1584, TC-1585, TC-1587, TC-2241..TC-2246; stubs in fsl-analyze, fsl-command and tests/it)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: reviews
---

## Summary

Ticket: QSL-2295. Structural checks: new ACs sit in their tables in sequence
(FR-640-AC-12..14, FR-641-AC-6..7, NFR-017-AC-5..7), each new TC has a
`verifies` edge to its FR, TC scopes match the AC ids, and there are no id
collisions with origin/main. The cross-requirement contradictions are in
base.md (FND-001, FND-002). One structural defect follows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both new `### Decision` headings sit in the middle of a bullet list. In FR-625 the remaining request-item members (`temporal.subject`, `temporal.limits` and those after) now fall under "Decision: an invariant item keeps its family's kind" instead of "Request item". In FR-627 the "Budgets and cancellation" and "Refusals" behaviour bullets now fall under "Decision: the supplied-trace path is the clause run's". A reader or a section lookup (`quire lookup`) files normative rules under the rationale. Fix: move each Decision subsection after the list it interrupts, at the end of "Inputs" and of "Behavior". | spec/functional/FR-625-request-and-report-an-explicit-state-model-check-item.md:45; spec/functional/FR-627-request-liveness-items-under-weak-and-strong-fairness.md:66 |

## Verdict

REQUEST CHANGES on FND-001, a layout fix.

## Dispositions

Round 2, verified at head a158bccc0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-625's `temporal.subject`, `temporal.limits` and `reductions` bullets now come before its Decision. FR-627's Budgets and Refusals bullets now come before its Decision. |
