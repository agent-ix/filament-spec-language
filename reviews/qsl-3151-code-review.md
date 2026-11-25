---
id: SR-967175026302197768900474846
title: "Code review of FSL PR #3798: fair eventual source/package parity retag"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@9518003073678e079c7a52b44a442a0f0c4135f4; PR #3798 against main; fsl-analyze/tests/it/package_probabilistic_refutation.rs, fsl-analyze/tests/it/timed_product_certificate/bounded_monitor_profile.rs; FR-200-AC-4/8/9/10, FR-940-AC-40..44, FR-741-AC-3, FR-203-AC-8, TC-2958 step 12"
review_set: subset
---
# Code review of FSL PR #3798

## Summary

Ticket: QSL-3151. The review pass (recorded on the ticket in Linear comment 4fa1d7e1) examined the retagged four-form fairness parity test and the weak-whole reverse-bound test against the FR-200 and FR-940 criteria and TC-2958 step 12. This file was rebuilt from that comment for the disposition pass because the original scratch copy was not available. The `## Findings` row below is the review pass's finding unchanged.

## Verdict

Review pass: one low trace finding. Disposition pass 1 is at head `9518003073678e079c7a52b44a442a0f0c4135f4`. FND-001 is fixed. Two new low trace findings remain: the reverse-bound test and TC-2958 step 12 still bind FR-200-AC-4, which is the section 15.4 `Deliver` criterion, to `Link` tests. The rustfmt-only change in `bounded_monitor_profile.rs` is correct; `rustfmt --edition 2021 --check` passes on both changed files. Not mergeable at this head until FND-002 and FND-003 are fixed or dispositioned.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The four-form parity test retains FR-200-AC-4, FR-741-AC-3 and FR-203-AC-8 tags without asserting their exact minimum/fairness-set or witness/replay clauses. | fsl-analyze/tests/it/package_probabilistic_refutation.rs:167 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | `a_fair_every_scheduler_refutation_replays_with_source_package_parity` still traces FR-200-AC-4. That criterion concerns the section 15.4 `Deliver` fixture with its 24/25 minimum and recorded fairness set; it is owned by the TC-635 test in exact_probabilistic.rs:917. This test runs the `Link` fixture at `<= 98/100`. Neither it nor `refutation_parity` asserts a 24/25 minimum or a fairness-set record. Remove the `FR-200-AC-4` tag. | fsl-analyze/tests/it/package_probabilistic_refutation.rs:229-236, 529-702 |
| FND-003 | low | TC-2958 step 12 tells both `Link` tests to bind the weak-whole form to FR-200-AC-4, the `Deliver` 24/25 fairness-set criterion. Following it inflates FR-200-AC-4 coverage, so the step contradicts the AC it names. Drop both FR-200-AC-4 bindings from step 12, leaving FR-940-AC-40 for the loop and FR-940-AC-44 for the reverse bound. | spec/test-cases/TC-2958-source-analysis-keeps-its-checked-graph-and-the-package-route-reaches-every-evaluator.md:87-94 |

## Dispositions

Round 1 at `9518003073678e079c7a52b44a442a0f0c4135f4`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 8bb3eb4a40 removed FR-200-AC-4, FR-741-AC-3, FR-203-AC-8, FR-200 and TC-2958 from the four-form test, and 96d818fadf restored TC-2958. The remaining tags are TC-2958, FR-200-AC-8/9/10 and FR-940-AC-40..43. The loop and its helper assert each of them: exit 10, `refuted`, equal exits and equal complete documents. |
