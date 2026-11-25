---
id: SR-2026112508191401
title: "base review of PR 3876 snapshot tuple and union components spelling"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-464-snapshots-and-invocations-admit.md; PR 3876 head 46864ca27adae0679c423bc8f07c89f86377e13e"
review_set: subset
---

## Summary

Ticket: QSL-3263. Base checklist over the two edited files (FR-106 closed value list, tuple and payload-bearing union paragraphs, FR-106-AC-84; TC-464 step 11 and its expected result). IDs, AC table shape and TC binding (step 11 to FR-106-AC-84) are intact; no hash, digest, pin or tracking-record use introduced. One low wording ambiguity in a TC-464 control.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-464 step 11 "a `Has` case with zero ... components" can be written as `"components":[]` (invalid-value) or with `components` omitted (missing-member per FR-106); the expected result requires invalid-value, so the control should say `"components":[]` | spec/test-cases/TC-464-snapshots-and-invocations-admit.md:126, spec/test-cases/TC-464-snapshots-and-invocations-admit.md:199-202 |

## Verdict

Base checklist passes apart from FND-001. FR-106-AC-84 is testable as written: every control it names is a single-defect value whose cause the FR-106 paragraphs determine. No test embeds either edited file (no include_str!/read of FR-106 or TC-464; the only spec-reading tests are for FR-255, FR-499, FR-500, FR-521, FR-531). No applicable enforced AssuranceProfile was found for this repo; direct review.

## Scope

- FR-106 closed value list (FR-106:393-400): examined.
- FR-106 tuple paragraph (FR-106:671-683): examined.
- FR-106 union paragraph (FR-106:685-704): examined.
- FR-106-AC-84 (FR-106:1596): examined.
- TC-464 step 11 and expected result (TC-464:113-130, 197-205): examined.
