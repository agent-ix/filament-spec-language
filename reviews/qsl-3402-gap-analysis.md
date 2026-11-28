---
id: SR-967183
title: "QSL-3402 gap analysis of the operation redefinition criteria (filament-spec-language PR 4004)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@a7a5e0be8bd68f13f03c8bff23483747c91f39f6; criteria FR-082-AC-1, FR-082-AC-5, FSpec-FR-151-AC-4, FSpec-FR-151-AC-21"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3402. PR filament-spec-language#4004. This analysis covers only
the criteria the diff tags or changes. The repo-wide matrix was computed with
`quire matrix --scope . --format json`, and only these rows were read.

- FR-082-AC-1: tagged. Binders are
  `r03_an_incompatible_operation_redefinition_reports_every_failing_axis`,
  `c01_r03_and_r08a_report_every_failure_in_member_axis_order`,
  `a_redefinition_that_widens_the_frame_refuses_effect_escape` and the root
  `operation_conformance_reports_each_independent_axis_and_all_three_together`.
  All of them ran on this head and passed.
- FR-082-AC-5: tagged. The binder
  `r04_an_arity_mismatch_refuses_without_checking_parameter_axes` ran on this
  head and passed.
- FSpec-FR-151-AC-4: an external criterion, so it is not in the FSL matrix.
  Six `FSpec-TC-196` tests in `model_conformance.rs` carry it, and all ran on
  this head.
- FSpec-FR-151-AC-21: an external criterion. The new
  `optional_parameter_presence_is_compared_by_the_parameter_type_axis` runs
  all four TC-2054 step 5 pairs through real intake and checking. It adds
  three further pairs that follow from FR-151's "`Option<X>` and `X` never
  conform", and it fails on the pre-fix code for the intended reasons. The
  edited bundle-h test also carries the tag, as partial evidence (code-review
  FND-003).

Reverse gap: the new `parameter_type` carries
`implements("FSpec-FR-151-AC-21")`. The new `ModelIndex::record` is an
untagged lookup helper with no behavior of its own. There is no code without
an owning requirement.

Stubs or coverage inflation: none. The new test asserts literal values (unit
total 10, axis `parameter-type`, code `IllTyped`, cause tag, member and
parent keys, index 2) with no oracle derived from the code under test.

## Verdict

PASS for the changed criteria. Every criterion is tagged by a test that ran
and passed on this head. Trace precision and spec ambiguities are recorded in
the code-review artifact for this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- Criteria examined: 4; tagged: 4; untagged: 0.
- Matrix: computed by `quire matrix`; FSpec criteria checked by grep for
  their `FSpec-` trace ids.
- Semantic review: skipped (not requested); the intent questions for
  FR-151-AC-21 are answered in the code-review artifact.
- Plan completion: not assessed
