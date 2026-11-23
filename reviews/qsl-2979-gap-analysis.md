---
id: SR-3599002
title: "gap-analysis of PR3599 settle each instance of a probabilistic on-each claim"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3599 5a77ef036cf4df145c532e41e0be3f586d124812; FR-875, FR-741-AC-5, FR-196 Conditioned readings, FR-201 Conditional certificates; tests in fsl-analyze/tests/it/probability_from_source.rs and tools/fsl-probability-check/tests/it/on_each.rs"
review_set: subset
---

## Summary

Ticket: QSL-2979. This is a diff-scoped gap audit over `quire matrix --scope . --format json`
at the PR head (635 requirements). FR-875-AC-1, -2 and -3 are tagged (TC-2485, TC-2486,
TC-2487), and so are FR-741-AC-5 and FR-186-AC-10. FR-196-AC-15 is tagged by the existing
conditioned_product_key.rs tests. FR-201-AC-52 and FR-201-AC-53 (TC-3061: the `Optimum`, the
`Vacuous` certificate and mode-misplacement rejection) are untagged and unbuilt. The five
target tests are un-ignored, and their assertions are unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-875 Behavior "Activation and conditioning" (the optimum over activating schedulers by the reset construction) and "Settling the item" (a statistical instance by its measurement, `measured`), with FR-201-AC-52/53 (TC-3061), are untagged and unbuilt. No ticket is named, and those items settle `unsupported`, which FR-875 forbids | fsl-replay/src/probability_certificate/document/instances.rs:88 |
| FND-002 | medium | FR-741-AC-5 is only partly covered. No test asserts the numerator `391/400` and denominator 1 *at `m`'s initial key*, or that `n`'s initial key is confirmed vacuous (its rows or its absence from `entries`). The tests check only that some row anywhere holds each value | fsl-analyze/tests/it/probability_from_source.rs:772-786 |
| FND-003 | low | `an_on_each_conditional_certificate_is_accepted_and_each_row_is_checked` is tagged `TC-2065`/`FR-201-AC-51`. AC-51 lists other certificate kinds (Exact, value-bounds, LongRun, fair MDP, quantile, digital-clock), and it requires a rejection "with its FR-201 rule and locus", which the test does not assert | tools/fsl-probability-check/tests/it/on_each.rs:41 |
| FND-004 | low | Reverse gap: production code carries a criterion id in prose (`restrict_spelled_states` cites FR-875-AC-3, which quire reports as an unmarked trace id), and the new wire profile `filament.probability-objective/on-each-reachability/v1` has no owning requirement | fsl-analyze/src/analyze/document.rs:4453 |

## Verdict

CONDITIONAL. There are no stubs and no hollow assertions in the new code. The five
un-ignored tests assert real verdicts, and both the empty-population and the first-failing
rerun checks are meaningful. FND-001 needs tickets, or the ticket narrowed by the owner.
FND-002 and FND-003 should be fixed in this PR's new test, not in the frozen five.

## Coverage

- FR-875-AC-1: tagged, probability_from_source.rs:692.
- FR-875-AC-2: tagged, probability_from_source.rs:709; over-asserts (see SR-3599001 FND-008).
- FR-875-AC-3: tagged, probability_from_source.rs:736.
- FR-741-AC-5: tagged, probability_from_source.rs:759 and on_each.rs:41; partial (FND-002).
- FR-186-AC-10: tagged, probability_from_source.rs:627.
- FR-196-AC-15: tagged, conditioned_product_key.rs:19, :40, :50.
- FR-201-AC-51: on_each.rs tag is a trace error (FND-003).
- FR-201-AC-52, FR-201-AC-53: untagged and unbuilt (FND-001).
- Semantic review: done in SR-3599001.
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `probabilistic_on_each_state_limit_settles_incomplete_naming_the_limit` is tagged `TC-3208`/`FR-741-AC-5`. TC-3208's scope is FR-201-AC-76/77, and FR-741-AC-5 says nothing about limits. The test checks FR-741 Budgets, which has no on-each AC | fsl-analyze/tests/it/probability_from_source.rs:1797 |
| FND-006 | low | `probabilistic_on_each_out_of_profile_form_is_unsupported_with_no_cause` is tagged `TC-3209`/`FR-201-AC-80`, but it uses a bounded `eventually[0,2]` form. AC-80 names "a cyclic continuation graph, with authored fairness or over a timed subject", and none of those clauses is tested. The test fits FR-875's "Out-of-profile forms (current)" bullet, which has no AC | fsl-analyze/tests/it/probability_from_source.rs:1814 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-2995 (the reset-construction Optimum, statistical evidence, cyclic graphs and other forms); FR-875's "(current)" bullet names the gap |
| FND-002 | still-open | FR-741-AC-5 and FR-201-AC-76's numerator `391/400` at `m`'s initial key is still asserted nowhere at that key (see SR-3599001 FND-010) |
| FND-003 | fixed | on_each.rs no longer carries the FR-201-AC-51 tag. Its tests are tagged FR-201-AC-76..80 (TC-3208/3209) and FR-741-AC-5 |
| FND-004 | fixed | `restrict_spelled_states` and its prose criterion id are removed, and the profile is owned by FR-201 "Closed on-each reachability conditional profile" (#3603) |
| FND-002 | fixed | Round 2: on_each.rs asserts `391/400` and `1` at `m`'s initial key and `0`/`0` at `n`'s (FR-201-AC-76, FR-741-AC-5) |
| FND-005 | still-open | Round 2 retagged the limit test to `TC-2066`/`FR-741-AC-2`. FR-741-AC-2 is about accepted certificates across forms (P95 ratio, value bounds, long-run ...), not a state limit, and no FR-741 AC covers an on-each `max_states` stop (FR-741-AC-6 is about work units). The binding is still wrong. Drop the AC tag and keep it as an untagged regression test, or add an FR-741 Budgets AC for it |
| FND-006 | fixed | Round 2: the analyze test now uses the AC-80 clauses (authored fairness, and every scheduler) and asserts `unsupported`/`not-yet-implemented`, no cause and no certificate. on_each.rs covers the cyclic, fair and timed clauses |
