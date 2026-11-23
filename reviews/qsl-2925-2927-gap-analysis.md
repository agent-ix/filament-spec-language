---
id: SR-967175026302197768900474181941935084788
title: "Gap analysis of QSL-2925/QSL-2927 fixed-N preflight and verdict bound"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3545 head a365faa13aa4fc5c3d5a41bc14b2e41f76f7c4f0 vs merge-base 9a2555a0; criteria FR-191-AC-1..8, FR-192-AC-1..9, FR-741-AC-4; FSpec FR-410-AC-12, FR-408-AC-19, FR-331-AC-67, FR-413-AC-17"
review_set: subset
---

## Summary

Tickets: QSL-2925 and QSL-2927. A planless gap analysis over the criteria the PR's governing specs amended (FSL #3547, FSpec #816) and the code in the PR diff.

`quire matrix --scope . --format json` did not finish in 150 s (timeout, exit 124; the only output was DuplicateArchetype warnings). The matrix below is therefore a grep index of `#[trace(...)]` tags over `*.rs`, excluding `spec/` and `target/`. It is not the computed matrix.

Plan completion: not assessed

Reverse gap: every new code path is owned by FR-191 (preflight) or FR-192/FR-410 (`bound`, wire `required`). No unowned behaviour was found. Stubs: none.

The reviewer also ran a semantic check of test intent against the ACs (no user opt-in prompt was available; the brief asked for test-quality judgement).

## Verdict

**FAIL** by the verdict rule: FR-191-AC-6, amended by #3547 to expect a preflight stop, has no tagged test. That criterion predates this PR: the native route runs serially (FR-741), the TC-1417 worker tests do not exist on main, and it does not gate #3545 alone. The PR-attributable findings are medium/low: FND-002, hollow FR-192-AC-9 coverage, must be fixed; FND-003 to FND-005 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-191-AC-6 has no tagged test. It was amended to "the Okamoto run is a PreflightLimit{MaxSamples, 1,000, required 9,210,341} with zero samples and draws" across workers 1/2/4/64, and no TC-1417 test exists in the repo. This predates the PR (workers are not executed), but the amended clause is untested | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md |
| FND-002 | medium | FR-192-AC-9 coverage is inflated. `okamoto_sprt_and_regenerative_results_settle_measured` was newly tagged FR-192-AC-9 (and stays FR-192-AC-2), but it asserts no `bound`. `a_verdict_carries_the_claims_bound` covers one accepted SPRT quantile verdict, and its threshold is compared against the same `claim.bound()` accessor the code reads. No test asserts `bound` on a rejected, undecided or undefined-evaluation verdict, and none covers the reader refusal. The from-source tests carry the `FSpec-FR-410-AC-12` tag but exercise only accepted verdicts and no refusal | fsl-analyze/src/statistical/tests.rs:1792-1811; fsl-analyze/src/statistical/tests.rs:1952-1954; fsl-analyze/tests/it/probability_from_source.rs:377-391 |
| FND-003 | low | FR-191-AC-1 says "with either bound at 9,210,341 it completes". Only `max_samples` = 9,210,341 is tested; the `max_draws` = N equality admission is untested | fsl-analyze/src/statistical/tests.rs:1141-1145 |
| FND-004 | low | FR-741-AC-4 says "with zero samples and draws", and the FR-192 preflight row says "partial basis zero". The incomplete wire has no partial counts, so the from-source tests assert only the cause, and their comment "nothing is drawn" is unasserted. Zero counts are checked only at the unit level | fsl-analyze/tests/it/probability_from_source.rs:1049-1052; fsl-analyze/tests/it/probability_from_source.rs:1063-1083 |
| FND-005 | low | The native preflight cause is not checked against the pinned FSpec schema: the verdict is validated against `StatisticalVerdict`, but the `limit-reached` cause with `required` is not validated against `LimitReached`. FSpec-FR-408-AC-19 and FSpec-FR-331-AC-67, which the output realises, carry no tags, so the out-of-range `required` (code-review FND-001) goes uncaught | fsl-analyze/tests/it/probability_from_source.rs:1063-1083; fsl-analyze/tests/it/probability_from_source.rs:1586-1590 |

## Coverage

- Grep index (tag count). FR-191-AC-1: 3, AC-2: 2, AC-3: 0, AC-4: 0, AC-5: 1, AC-6: 0, AC-7: 0, AC-8: 2. FR-192-AC-1: 8, AC-2: 6, AC-3: 1, AC-4: 0, AC-5: 1, AC-6: 4, AC-7: 1, AC-8: 0, AC-9: 3. FR-741-AC-4: 4. FSpec-FR-410-AC-12: 4. FSpec-FR-408-AC-19, FSpec-FR-331-AC-67, FSpec-FR-413-AC-17: 0 (FSpec's TC-355/TC-431).
- Untagged and outside this diff (predate the PR, not findings here): FR-191-AC-3/4, whose tests carry only `TC-626` with the AC in a doc comment; FR-191-AC-7 (workers); FR-192-AC-4; FR-192-AC-8.
- Bindings checked: max_samples_stops_a_run_before_its_method_completes, FR-191-AC-1, correct. max_draws_and_cancellation_stop_with_no_decision, FR-191-AC-2, correct. max_samples_stops_before_the_next_draw_and_stops_carry_provenance, FR-191-AC-5, correct. clopper_pearson_compares_its_sample_count_with_the_budgets_before_drawing, FR-191-AC-1, correct. a_tiny_indifference_region_settles_incomplete_before_sampling, FR-741-AC-4 and FR-191-AC-1, correct; FR-192-AC-9 partial. okamoto_sprt_and_regenerative_results_settle_measured, FR-192-AC-9, wrong (FND-002). statistical_*_from_source, FR-192-AC-2, partial: a different claim from AC-2's.
- Semantic review: by the reviewer, over the changed tests only.

## New findings (disposition pass 1)

Re-review of head 31645dfc36813a83879e598d85fe71cf3b28fe9d. Cargo was again not run: eight other cargo jobs were running on the host.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The from-source quantile path is not exercised end to end. `model.rs` now captures the written `(q, comparison, c)`, but no source quantile claim (in particular `>= c`) asserts its verdict `bound` with `level`. No undefined-evaluation rejection asserts its `bound`, which FR-410 requires there too. The unit tests build the claim with `with_quantile` by hand | fsl-analyze/src/statistical/model.rs:626-680; fsl-analyze/src/statistical/tests.rs:1838-1908 |
| FND-007 | low | `a_tiny_indifference_region_settles_incomplete_before_sampling` still carries the whole `FR-192-AC-9` tag. That AC's reader clause is deferred to QSL-2944, and its bound clause is tested under FR-192-AC-2, so the matrix shows AC-9 as covered | fsl-analyze/tests/it/probability_from_source.rs:1063 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-2943. FR-191-AC-6 (workers) is specified and not built. It is left untagged, with a comment in tests.rs |
| FND-002 | fixed | The wrong FR-192-AC-9 tag is removed. `a_verdict_carries_the_claims_own_bound` asserts accepted (quantile `<= 5`), rejected (quantile `>= 2`) and undecided (long-run `<= 1999/2000`) bounds with independently written exact values (level 19/20) |
| FND-003 | fixed | `a_max_draws_equal_to_the_sample_count_admits_the_run`: `max_draws` 23,026 completes with (23026, 23026); 23,025 stops with PreflightLimit, zero counts and zero calls |
| FND-004 | rejected | The FR-331 incomplete cause has no partial-count member by design, so zero samples and draws cannot be observed on the wire. The unit tests assert them (probe.calls == 0), which is the right level |
| FND-005 | fixed | Two schema tests were added. The preflight cause is written by the real pipeline and validated against the pinned `LimitReached`, with exact `"921034038"`. The bound is serialized by the single writer `verdict_bound_wire` and validated against `ProbabilisticBound`, with exact members per form. Neither is a tautology: the schema comes from FSpec |
