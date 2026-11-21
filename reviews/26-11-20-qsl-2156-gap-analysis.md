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
| FND-002 | fixed | At 785bc280a0, `every_scheduler_discrete_delay_is_certified` carries `#[trace("TC-2492")]` only; the computed matrix (quoin 0.36.8) now reports FR-923-AC-1 `tagged-by-ignored-test` via `every_scheduler_discrete_delay_refutes`, and the PR body lists the refutation gap. |
| FND-003 | fixed | `exact_certificate_rejects_threshold_miss_and_cycles` asserts `Err(Cycle)` for Minimum, Maximum and Workload under `GraphCycles::Reject`, and `an_untimed_wait_loop_cannot_certify_a_scheduler_minimum` (TC-636, FR-201) asserts `Err(Cycle)` for a forged {A:1,T:1} wait loop under Minimum and Maximum. |

## Disposition pass summary

At frozen head `2b16a60fef4b7382fecdf1f9148961c0baeaecfa`, FR-921-AC-4 remains parked owed-test evidence under the arch scope freeze. No follow-up ticket was created or deferred.

## New findings (disposition pass 2)

Planless, PR-scoped, at head `b42bed4b90`. Plan completion: not assessed. Matrix: `quoin matrix --repo /home/peter/dev/worktrees/qsl-2156-probability-digital --json`, quoin 0.36.8, `spec-artifacts-process@v0.36.0`, exit 0. No cargo run in this pass; the run evidence is the lead's `/tmp/qsl2156-focused-3.log` (exit 0).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | FR-923-AC-1 shows `tagged` only because of its proof half. The refutation half (`every_scheduler_discrete_delay_refutes`, bound above 159129/160000 refuted) is ignored, so the matrix reports the criterion as covered when half of it has no active evidence. | fsl-analyze/tests/it/timed_product_certificate.rs:493-499 |
| FND-003 | medium | FR-201's unbounded acyclic restriction has no active test. After the fix round, the only cycle case (`exact_certificate_rejects_threshold_miss_and_cycles`, TC-636) asserts the `Workload` refusal only, so the untimed every-scheduler cycle acceptance (code-review FND-002) is untested. | fsl-replay/src/probability_certificate.rs:2417-2447 |

### Coverage at this head

- Active: FR-921-AC-1..3, FR-922-AC-1..3, FR-923-AC-1 (proof half), FR-923-AC-2, FR-924-AC-1, FR-924-AC-2, FR-924-AC-4, FR-198-AC-1..5.
- `tagged-by-ignored-test` (the PR body lists these as remaining QSL-2156 scope): FR-877-AC-1..4, FR-923-AC-3, FR-924-AC-3, FR-924-AC-5, FR-924-AC-6, and FR-921-AC-4 (prior FND-001, deferred).
- No timed deadline refutation is exercised by any active test. The `!holds` branch of `run_timed` (`fsl-analyze/src/analyze/exact.rs:342-380`) has no active evidence.
- Reverse gap and stubs, in the changed production paths only: no unowned behaviour and no empty implementation found. The test helpers changed in the fix round (`digital_clocks_of`, the `Max` bound) keep their oracles; see the code-review artifact.

## Disposition pass 2 summary

At head `b42bed4b90`, prior FND-001 (FR-921-AC-4) stays `tagged-by-ignored-test`. Its latest outcome is `deferred`, so no new row is due. FND-002 and FND-003 are new and have no outcome yet.

## Disposition pass 3 summary

Head `785bc280a0`, matrix `quoin matrix --repo . --json` (quoin 0.36.8, `spec-artifacts-process@v0.36.0`, exit 0). FND-002 and FND-003 fixed. FND-001 (FR-921-AC-4) stays deferred and is now listed in the PR body's remaining work. Remaining `tagged-by-ignored-test` criteria (FR-877-AC-1..4, FR-921-AC-4, FR-923-AC-1, FR-923-AC-3, FR-924-AC-3, FR-924-AC-5, FR-924-AC-6) are all listed as remaining QSL-2156 scope; no new gap finding.
