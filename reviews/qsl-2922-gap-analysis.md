---
id: SR-967175026302197768900474181941935084769
title: "Gap analysis of PR3544 exact fraction-window work fix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3544 55779853a92c576237e176f10f6e227028021518; FR-203, FR-120 cumulative work, TC-638; fsl-replay/src/probability_certificate/document/measure.rs; fsl-analyze/tests/it/exact_probabilistic_item/fraction_window.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-203
    type: references
---

## Summary

Ticket: QSL-2922. Planless gap analysis scoped to the PR's diff: the criteria its new tests cite (TC-638,
FR-203-AC-1), the criteria governing the changed behaviour (FR-120 cumulative work, FR-203-AC-13), and
measure.rs `enumerate`. The change adds no spec and no new public surface. A grep trace index (the computed matrix did not finish) shows
FR-203-AC-1 traced by `p95_settles_proved_with_its_exact_value`, which genuinely exercises it. The two new tags add
coverage that is not real. The new caching rule, under which a model state is expanded and charged once per
enumeration however many product states stand over it, has no owning requirement text.

## Verdict

CONDITIONAL. No criterion loses its only real test, and no stubs or placeholders were found in the changed code.
The real implementation runs under the new tests, and they fail on origin/main. Two medium/low findings remain:
the AC-1 tags inflate coverage, and the expand-once work rule is unspecified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Coverage inflation: two tests that settle a window-10 fraction mean and a work-units boundary are traced to FR-203-AC-1, whose criterion is §15.2 P95 ExactValue{24233/25000}, §15.1 ValueBounds and §15.4 Deliver refuted | fsl-analyze/tests/it/exact_probabilistic_item/fraction_window.rs:30,49 |
| FND-002 | low | Reverse gap: the user-visible work count of an exact measure item now depends on expanding each model state once per enumeration (265,325 vs about 3.47M units). No FR or AC states this charge rule or a work bound that a fraction-mean claim must meet, so the boundary test pins an unspecified number | fsl-replay/src/probability_certificate/document/measure.rs:676-686 |

## Coverage

- Criteria examined: FR-203-AC-1 (traced by the existing P95 test and, wrongly, by the two new tests), FR-203-AC-13 (traced by probability_from_source.rs tests; not cited by this PR), FR-1043-AC-5 (fraction-mean ExactValue; not cited by this PR).
- Matrix: `quire matrix --scope . --format json` was started but did not finish in about 15 minutes on this repository and was stopped. A grep index of `#[trace]` tags (declared fallback) was used for the cited criteria: FR-203-AC-1 is tagged by `p95_settles_proved_with_its_exact_value` plus the two new tests, and FR-203-AC-13 by probability_from_source.rs tests. Whole-repo untagged-criterion status was not assessed.
- Reverse gap: measure.rs `enumerate` caching, see FND-002.
- Stubs: none (no `todo!`, `unimplemented!`, placeholder returns or tautological assertions in the diff).
- Semantic review: skipped (not opted in).
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | New test `a_restored_named_state_is_not_expanded_from_an_anchored_state_with_equal_bytes` is traced to FR-202-AC-28, whose criterion is the `11/48`/`5/12` minimum and maximum refutations and the `Malformed` at `kind` role rejections over the fast/slow model; the test asserts an `Expansion` refusal (`missing-required-artifact`) for a restored state on a `retry` model and checks none of that | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:3076 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Both tests are untagged from FR-203-AC-1. The boundary test now traces `TC-638`/`FR-203-AC-13` (cumulative `work_units` stop at B, asserted at B and B-1). The fraction-mean test now traces `TC-3152`/`FR-1043-AC-5` (fraction mean settles `proved` with `ExactValue`), as a contributing binding on a longer window. |
| FND-002 | deferred | No spec edit in this PR, which is correct for a code fix. `grep` of spec/functional and spec/non-functional confirms that no FR or AC states that an exact measure checker expands each model state once, or bounds the work of a fraction-mean claim. FR-120 "Cumulative probabilistic work" only forbids replenishing the allowance, and FR-047-AC-5 states the analogous rule for object graphs only. The test file now says the boundary pins current accounting and no requirement states it. Follow-up: a small FR-1043 spec ticket (text in the QSL-2922 dispositions comment). |
