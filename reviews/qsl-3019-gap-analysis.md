---
id: SR-967175026302197768900474181941935084851
title: "Gap analysis of FSL PR #3885: compensation outcome refinement rows"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@7508fadd789835811f58886d1a769e587321c5a0; PR #3885 against origin/main; FR-136, FR-147-AC-8, FR-148-AC-1 and the FSpec criteria the PR traces (FR-432-AC-8, FR-432-AC-25, FR-430-AC-4, FR-375-AC-23)"
review_set: subset
---

## Summary

Ticket: QSL-3019. Computed `quire matrix --scope . --format json` at head (`sr-3885/matrix-head.json`).

- Every FR-147 and FR-148 criterion the PR touches is tagged. FR-147-AC-8 has 4 binders, including the new `the_cfail_ctimeout_and_cexpire_classes_resolve`. FR-148-AC-1 has 8, including the five new product tests.
- FR-136-AC-5, FR-136-AC-7 and FR-147-AC-3 are untagged at head and equally untagged on main. They predate this PR and the PR does not touch them, so they are not findings here.
- Reverse gap: two production changes have no FSL criterion of their own: the emission fix (`lowering/protocol.rs`) and the fsl-eval fairness filters. Both are traced to FSpec criteria (FR-375-AC-23, FR-430 and FR-430-AC-4). That is acceptable under the repo's `FSpec-` tag rule.
- The new FR-136 template-row requirement has no FR-136 criterion; see the spec-review SR.
- Stub scan of the diff found no `todo!`, `unimplemented!` or placeholder arms.

Plan completion: not assessed

## Verdict

**CONDITIONAL.** There are no untagged criteria in scope. One low finding: an FR-level-only tag. Semantic review was not requested, so it was skipped.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `an_operation_fairness_class_holds_attempts_and_compensation_attempts_only` is tagged at statement level, `FSpec-FR-430`, with no criterion id, so the computed matrix cannot bind it to an FR-430 AC. Its sibling test names `FSpec-FR-430-AC-4`. | fsl-analyze/tests/it/protocol_compensation.rs:2225 |

## Coverage

- Matrix: computed at head; FR-136 (13 ACs, 2 untagged pre-existing), FR-147 (16, 1 untagged pre-existing), FR-148 (14, all tagged).
- Semantic review: skipped (not requested).
- Plan completion: not assessed
