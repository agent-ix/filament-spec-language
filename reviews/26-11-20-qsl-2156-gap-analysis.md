---
id: SR-2026112013172122021003266
title: "Gap analysis of QSL-2156 PR #3204 timed digital certificates"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2156; PR #3204 head 85ff4ce2e72ebebcf220671e0ab96f43fc48d49a; timed-certificate criteria and changed production paths"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---
# Gap analysis: QSL-2156 PR #3204

## Summary

Planless PR-scoped review. Plan completion: not assessed. The computed repository matrix ran at the single worktree root. It contains 6,037 criteria overall; 1,170 have no active trace binder. The timed slice's FR-921/922/923/924 criteria are tagged, except FR-921-AC-4 is tagged only by an ignored test. FR-198-AC-1 through AC-5 are tagged. The source and tests changed by this PR contain no empty implementation or unconditional test stub. The full repository reverse-gap inventory was not performed; this is a PR-scoped analysis, not a repository-wide assurance claim.

- FR-921-AC-1..3 are backed by the corresponding `timed_product_certificate` tests.
- FR-921-AC-4's fresh-process checker case has only an ignored binder in the computed matrix; no active evidence confirms the independent binary path for the new timed certificates.
- FR-922-AC-1..3 and FR-923-AC-1..3 are tagged by the changed timed-product test suite.
- FR-924-AC-1..6 are tagged by the changed timed-product test suite. Static inspection found the region checker still refuses pending quiescent states; see the code-review finding SR-2026112013172122021003265/FND-001.
- No unowned behavior or source/test stub was found in the changed production paths. Production code remains under the timed-certificate requirements; the new integration cases exercise analyzer and checker paths.

## Verdict

FAIL for the focused assurance slice: FR-921-AC-4 lacks active fresh-process evidence. This is a test-evidence gap, not a new ticket request; the FSL scope rule currently parks owed-test work. Separately, FND-001 in the code-review artifact records the specified region time-lock behavior still refused by the checker.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix marks FR-921-AC-4 as `tagged-by-ignored-test`; the independent fresh-process verifier has no active test evidence for the timed certificates. | spec/functional/FR-921-check-a-timed-product-certificate-without-the-engine.md:96; tools/fsl-probability-check/tests |

## Coverage

- Matrix: `quoin matrix --repo . --json`, run from `/home/peter/dev/worktrees/qsl-2156-probability-digital`; quoin 0.36.8, `spec-artifacts-process@v0.36.0`. FR-921-AC-4 is tagged only by an ignored test. No `cargo` build or test was run, per the lead's explicit hold.
- Reverse-gap and stub review: changed production paths and changed tests only; no unowned behavior, source stub, or hollow test found in that subset. The complete source-tree inventory was not assessed.
- Test/criterion bindings inspected: TC-2490 -> FR-921-AC-1..3; TC-2491 -> FR-922-AC-1..3; TC-2492 -> FR-923-AC-1..3; TC-2493 -> FR-924-AC-1..6; TC-2494 -> FR-877-AC-1..4; TC-1404 -> FR-198-AC-5.
- Plan completion: not assessed.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | The arch scope freeze parks owed-test work. Record this evidence as parked; no follow-up ticket is being created or deferred. |

## Disposition pass summary

At frozen head `2b16a60fef4b7382fecdf1f9148961c0baeaecfa`, FR-921-AC-4 remains parked owed-test evidence under the arch scope freeze. No follow-up ticket was created or deferred.
