---
id: SR-202611141459003
title: "Criterion strength review of the QSL-1963 FR-495 Kernel-mode criteria"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/quire-spec-language; PR #2707 at 3b0bc910d964198a45d79d8f7c6f4489b93fc179, diff origin/main...HEAD: FR-495-AC-10, FR-495-AC-11, FR-495-AC-12, FR-495-AC-13"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-495, type: references }
---

## Summary

Ticket: QSL-1963. Jev (typesafe.ai System One) is not installed on this host (`jev` is not on
PATH), so this lens was judged by hand over the spec text and the PR #2706 code recorded in
SR-202611141459001.

## Verdict

PASS for AC-10, AC-11 and AC-12, with one low finding on AC-13. Each criterion names the
mode-by-mode refusal code and cause, and AC-11 and AC-12 also name the location or the number
of refusals, so a wrong implementation fails it:
- AC-10 fails if Kernel keeps the presence obligation, if Linked drops it, or if a guarded
  `value` is refused.
- AC-11 fails if Kernel reaches a package state clause.
- AC-12 fails if the measure is walked under the body's mode, if Kernel stops the termination
  phase, or if Kernel stops recording calls once it drops an obligation. The `f(x) + 1` path
  `[0]` separates "call recorded" from "walk stopped".
- AC-13's rational half would fail if the Kernel shape guard were removed, because the
  Kernel filter would then drop `Unproved(RationalRange)`. It cannot be built as worded,
  which is recorded once, as SR-202611141459001 FND-001, and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-13's two dispatch halves cannot fail under any Kernel defect in the definedness walk. `PackageDeclarations::check` makes both refusals (`TableOutOfRange`, `FunctionOutOfRange`) up front, before any body walk and whatever the mode, and their cause is not `Unproved`, so the Kernel filter cannot drop them. The criterion catches only a future change that gates the up-front validator on the mode. | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md:245 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
